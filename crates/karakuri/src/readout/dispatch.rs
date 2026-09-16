//! Pointer event translation and bay event dispatch methods.

use karakuri_console::egui;
pub(crate) use karakuri_operation::gate::{Class, Open};
use karakuri_operation::Output;

use super::*;
use crate::{ir_layer, node_addr, refusal, showing};

impl Readout {
    /// The one place a pointer event is routed, and the only place this file
    /// decides anything about input.
    ///
    /// Returns who the event belonged to. The caller's whole job with the answer is
    /// to hand the event to `egui` when it is [`Claim::Egui`] and not when it is
    /// not — see `karakuri_console::input` for the rule and for why it is written
    /// there rather than here.
    ///
    /// It is a method rather than four arms in `window_event` so that a gesture can
    /// be driven without a window: `winit` cannot be asked for an `ActiveEventLoop`
    /// outside its own loop, so an event handler is not something a test can call,
    /// and the part worth testing is this.
    pub(crate) fn pointer(&mut self, ctx: &egui::Context, event: Pointer) -> (Claim, Acted) {
        let at = match event {
            Pointer::Moved(p) => p,
            _ => self.panel.cursor(),
        };
        // Asked **before** anything acts. `released` takes the drag out of
        // hand, so a claim asked after it would see no drag, route the release
        // to `egui`, and hand `egui` a button-up it never saw the button-down
        // for.
        // **`mut` for the wheel arm alone.** Every other event's answer is
        // this call's; a wheel asks a second question of `input::wheeled`,
        // whose `Some` widens the claim to the panel — see that arm.
        let mut claim = claim(&mut self.panel, ctx, &self.view, at);
        let mut did = Acted::Nothing;
        match (event, claim) {
            // The panel learns where the pointer is either way — every
            // keyboard operation is addressed to it — and drags if something
            // is in hand. Whether `egui` is also told is the claim.
            //
            // **A move is what a fader emits on**, so this is the one arm that
            // can act without a button, and it acts whoever the claim went to:
            // rule 1 has already given the panel any drag in hand.
            //
            // **Asked before the move, and only with a fader in hand.** An
            // `Emitted(None)` is *a fader that did not change*, which is owed
            // no frame; a plain pointer move with nothing in hand is
            // `Change::Pointer`'s business and is owed one whenever the panel
            // claimed it, because that is the resize cursor going on and off.
            // Answering `Emitted(None)` for both would take the cursor with it.
            (Pointer::Moved(p), _) => {
                let fading = matches!(self.panel.in_hand(), Some(InHand::Fader));
                let operation = self.moved(p);
                if fading {
                    did = Acted::Emitted(operation);
                }
            }
            // **A press the panel claimed is on one of the console's
            // controls or on the panel itself**, and they are asked first for
            // the reason `claim` asked them last: rule 2 has already had its
            // refusal, so a press that got here and is on a control is that
            // control's. Every one of them is the same call `claim` made —
            // asked again, not copied. How many there are is
            // `karakuri_console::input::CONTROLS`, which is why this sentence
            // no longer says a number: it went stale four times.
            //
            // **The bay is derived once and asked five times**, exactly as
            // `claim` does it: a knob, a blend chip, a tally chip, a mask mini
            // and the strip they sit in are five questions about one laid-out
            // strip, and five derivations would be five answers.
            (Pointer::Down, Claim::Panel) => {
                self.panel.solve();
                // Decomposed pointer press dispatch across modal overlays and individual bays.
                // Each bay handler returns `Some(Acted)` if it claims the press, or `None` to pass through.
                let did = self
                    .dispatch_modal_press(ctx, at)
                    .or_else(|| self.dispatch_transport_press(ctx, at))
                    .or_else(|| self.dispatch_inspector_press(ctx, at))
                    .or_else(|| self.dispatch_head_press(ctx, at))
                    .or_else(|| self.dispatch_library_press(ctx, at))
                    .or_else(|| self.dispatch_staging_press(ctx, at))
                    .or_else(|| self.dispatch_transition_press(ctx, at))
                    .or_else(|| self.dispatch_sequencer_press(ctx, at))
                    .unwrap_or_else(|| self.dispatch_mixer_and_master_press(ctx, at));
                return (claim, did);
            }
            // **Where a drop names its deck**, and there are two sets of
            // rectangles it can name it by. A carry is the one gesture here
            // whose destination is not known until the button comes up, so the
            // bays are laid out *now* and asked which of their rectangles the
            // pointer is over — `Mixer::dropped` and `ProgramBay::dropped`,
            // the same derivations the frame drew and the same ones `claim`
            // hit-tests the knobs, chips and cells of. They are asked only
            // with a carry in hand: a boundary and a fader each come to rest
            // without a destination, and laying the mixer out on every release
            // would be two galley lookups per strip to answer a question
            // nobody asked.
            //
            // **The order is arbitrary and cannot matter**: the strips are in
            // the right pane and the cells in the centre column, so a point
            // inside one set is outside the other, and `or_else` is two
            // questions about one point rather than a precedence.
            //
            // **The cell half is asked with the deck's slot count**, which is
            // the strips this console was handed: the row is always `DECKS`
            // cells and a deck holds one to four slots, so a release on the
            // fourth cell of a three-slot deck has nothing to load into and is
            // refused exactly as `3` is refused from the keyboard
            // (`pointed`, and `View::select` under it) —
            // [ADR-0273](../../../docs/adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md).
            (Pointer::Up, Claim::Panel) => {
                let onto = match self.panel.in_hand() {
                    Some(InHand::Carrying) => mixer_bay(ctx, self.panel.layout(), &self.view.mixer)
                        .as_ref()
                        .and_then(|bay| bay.dropped(at))
                        .or_else(|| {
                            program_bay(self.panel.layout(), self.view.canvas)
                                .as_ref()
                                .and_then(|cells| cells.dropped(at, self.view.mixer.len()))
                        })
                        .map(Landing::Deck)
                        // And the third set of rectangles: the chain's own
                        // list, which takes a `kind L5` row and refuses any
                        // other with the reason
                        // ([ADR-0273](../../../docs/adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)).
                        // What the carried row is, is the view's reading —
                        // `View::chain_landing` resolves the name against the
                        // library's `kind L5` rows.
                        .or_else(|| {
                            let adding = self.view.chain_choices();
                            master_row(
                                ctx,
                                self.panel.layout(),
                                self.view.master_out,
                                self.view.master_chain.as_ref(),
                                &adding,
                            )
                            .as_ref()
                            .and_then(|row| row.dropped(at))?;
                            let carried = self.panel.carried()?;
                            Some(Landing::Chain(self.view.chain_landing(carried)))
                        }),
                    _ => None,
                };
                did = self.released(onto);
            }
            // **A wheel is routed by region rather than by control**, which
            // is `input::wheeled` and not `input::claim` — see that function
            // for why the two are separate questions. The claim it hands back
            // is a *second* answer to who the event belongs to and it can only
            // widen the first: `claim` gives a wheel to the panel while a drag
            // is in hand, `wheeled` gives it to the panel while the pointer is
            // over a pane or over the Library bay, and neither takes one away.
            //
            // **Two regions scroll and `wheeled` is the one place that says
            // which** — an Inspector pane by index, and the Library bay, which
            // is the only one of itself (ADR-0312). Each arm calls that
            // region's own `scroll`; nothing here decides which region a point
            // is in, for `input`'s own reason: the derivation that draws it is
            // what answers.
            //
            // **`Acted::Pointed` and not `Acted::Nothing`**, for the reason
            // that variant exists: a console pointer moved and no operation
            // was named. It is what tells `window_event` a frame is owed —
            // a wheel spun against the top of a list is `Nothing` and earns
            // none, whichever of the two it was over.
            (Pointer::Wheel(by), _) => {
                if let Some(turned) = wheeled(&mut self.panel, &self.view, at) {
                    let moved = match turned {
                        Turned::Pane(pane) => self.view.scroll_by(pane, by),
                        Turned::Library => self.view.scroll_library_by(by),
                    };
                    if moved {
                        did = Acted::Pointed;
                    }
                    claim = Claim::Panel;
                }
            }
            // **The secondary button, and the whole of what it reaches on
            // this panel is a row of the Library bay's list.** A press on one
            // puts that row's menu down; a press anywhere else asks for
            // nothing at all and is not an error — `menu_ask` answers `None`
            // and this arm leaves `did` as `Acted::Nothing`, which is what a
            // press on a bay's ground already does.
            //
            // **One call for both halves of the gesture**, which is
            // `LibraryBay::menu_ask`'s own shape: with no card down it asks
            // *which row did this name*, and with one down it asks *which item
            // did this pick* — so a secondary press while the menu is open
            // picks or dismisses exactly as a primary one does, and the
            // gesture does not care which button ends it.
            //
            // **The claim is asked the same way and is not this button's
            // question**: `input::claim` decides whose an event is from where
            // the pointer is, so a secondary press over a row is the panel's
            // for the reason a primary one is, and one over a boundary or over
            // nothing is not.
            (Pointer::Secondary, Claim::Panel) => {
                self.panel.solve();
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
                if let Some(ask) = picked {
                    did = self.menued(ask);
                }
            }
            (Pointer::Down | Pointer::Up | Pointer::Secondary, _) => {}
        }
        (claim, did)
    }

    // =========================================================================
    // Decomposed per-bay press dispatch helpers (P45)
    // =========================================================================

    /// Dispatches pointer presses to open modal cards and pulldown overlay menus.
    ///
    /// Per Rule 2 (ADR-0311), while a modal overlay is open, every press on the console
    /// belongs to it, and a press outside its bounds dismisses the overlay.
    fn dispatch_modal_press(&mut self, ctx: &egui::Context, at: Point) -> Option<Acted> {
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

        // Audio-in device selection pill and card
        let listing = audio_in_pill(
            ctx,
            self.panel.layout(),
            self.view.transport,
            self.view.audio.as_ref(),
        );
        let heard = listing
            .as_ref()
            .zip(self.view.audio.as_ref())
            .and_then(|(pill, audio)| pill.ask(audio, at));
        if self.view.audio.as_ref().is_some_and(AudioIn::open) {
            return Some(self.listened(heard.unwrap_or(AudioAsk::Shut)));
        }
        if let Some(ask) = heard {
            return Some(self.listened(ask));
        }

        // Arrangement preset pill and card
        let pill = arrangement_pill(
            ctx,
            self.panel.layout(),
            self.view.transport,
            self.view.audio.as_ref(),
            self.view.tracker,
            self.view.map.as_ref(),
            &self.view.arrangement,
        );
        let asked = pill
            .as_ref()
            .and_then(|pill| pill.ask(&self.view.arrangement, at));
        if self.view.arrangement.open() {
            return Some(self.arranged(asked.unwrap_or(Ask::Shut)));
        }
        if let Some(ask) = asked {
            return Some(self.arranged(ask));
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

        None
    }

    /// Dispatches pointer presses in the Transport bay.
    fn dispatch_transport_press(&mut self, ctx: &egui::Context, at: Point) -> Option<Acted> {
        let group = tracker_group(
            ctx,
            self.panel.layout(),
            self.view.transport,
            self.view.audio.as_ref(),
            self.view.tracker,
        );
        let tracked = group.as_ref().and_then(|group| {
            group
                .tapped(at)
                .or_else(|| group.octave(at))
                .or_else(|| group.nudge(at))
        });
        if let Some(operation) = tracked {
            return Some(Acted::Emitted(Some(operation)));
        }

        let look = look_row(
            ctx,
            self.panel.layout(),
            self.view.transport,
            self.view.audio.as_ref(),
            self.view.tracker,
            self.view.map.as_ref(),
            &self.view.arrangement,
            self.view.look,
        );
        let tone = look.as_ref().and_then(|row| row.tonemap(at));
        let exposure = look.as_ref().and_then(|row| row.exposure(at));
        if let Some(operation) = tone.or(exposure) {
            return Some(Acted::Emitted(Some(operation)));
        }

        if let Some(pill) = view::learn_pill(
            ctx,
            self.panel.layout(),
            self.view.transport,
            self.view.audio.as_ref(),
            self.view.tracker,
            self.view.map.as_ref(),
            self.view.learn,
        ) {
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

        if view::map_pill(
            ctx,
            self.panel.layout(),
            self.view.transport,
            self.view.audio.as_ref(),
            self.view.tracker,
            self.view.map.as_ref(),
        )
        .is_some_and(|row| row.pill.contains(egui::Pos2::new(at.x, at.y)))
        {
            return Some(Acted::Nothing);
        }

        let row = transport_row(ctx, self.panel.layout(), self.view.transport);
        let recording = row.as_ref().and_then(|row| row.record(at));
        if let Some(operation) = recording {
            return Some(Acted::Emitted(Some(operation)));
        }

        let tempo = row.as_ref().and_then(|row| row.tempo(at));
        if let Some(operation) = tempo {
            return Some(Acted::Emitted(Some(operation)));
        }

        None
    }

    /// Dispatches pointer presses in Inspector bay panes.
    fn dispatch_inspector_press(&mut self, ctx: &egui::Context, at: Point) -> Option<Acted> {
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

        None
    }

    /// Dispatches pointer presses to bay grips, Program head solo, and MCP class pills.
    fn dispatch_head_press(&mut self, ctx: &egui::Context, at: Point) -> Option<Acted> {
        if let Some(head) =
            program_head(ctx, self.panel.layout(), self.view.opening).filter(|head| head.hit(at))
        {
            return Some(Acted::Operated(self.soloed(head.op())));
        }

        if let Some(grip) = REGIONS.iter().find_map(|region| {
            bay_grip(self.panel.layout(), region.name).filter(|grip| grip.hit(at))
        }) {
            return Some(Acted::Operated(self.folded(grip.op())));
        }

        if let Some(pill) = Class::ALL.iter().find_map(|class| {
            mcp_pill(ctx, self.panel.layout(), *class, self.view.opening)
                .filter(|pill| pill.hit(at))
        }) {
            return Some(self.opened(&pill));
        }

        None
    }

    /// Dispatches pointer presses in the Library bay.
    fn dispatch_library_press(&mut self, ctx: &egui::Context, at: Point) -> Option<Acted> {
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
    fn dispatch_staging_press(&mut self, ctx: &egui::Context, at: Point) -> Option<Acted> {
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
    fn dispatch_transition_press(&mut self, ctx: &egui::Context, at: Point) -> Option<Acted> {
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
    fn dispatch_sequencer_press(&mut self, ctx: &egui::Context, at: Point) -> Option<Acted> {
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
    fn dispatch_mixer_and_master_press(&mut self, ctx: &egui::Context, at: Point) -> Acted {
        let sink = outputs(ctx, self.panel.layout(), self.view.opening)
            .map(|row| row.told(self.view.projector))
            .and_then(|row| row.chip_at(at).map(|output| (row, output)));
        let bay = mixer_bay(ctx, self.panel.layout(), &self.view.mixer);
        let adding = self.view.chain_choices();
        let master = master_row(
            ctx,
            self.panel.layout(),
            self.view.master_out,
            self.view.master_chain.as_ref(),
            &adding,
        );
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
        let tally = bay.as_ref().and_then(|bay| bay.tally(at));
        let mask = bay.as_ref().and_then(|bay| bay.mask(at));
        let chose = master.as_ref().and_then(|row| row.chose(at, &adding));
        if let Some(chose) = chose {
            return self.chain_chose(chose);
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

    /// A press on the audio-in pill or on its card, and what this program does
    /// about it.
    ///
    /// [`Readout::arranged`]'s shape one pill to the left, and the split is the
    /// same: the two answers that are the *control's* own state are performed here,
    /// and the one that is an operation leaves as one.
    ///
    /// Opening the card is where the host is read, and it is the only place: a
    /// listing of a machine's inputs is a device enumeration, which is not a thing
    /// to do on a frame path (P-0091) — the same rule under which the Library bay's
    /// names and the arrangement pill's are read on a press. So the list a hand is
    /// about to read is the list as of the press that opened it, an interface
    /// plugged in a minute ago included.
    pub(crate) fn listened(&mut self, ask: AudioAsk) -> Acted {
        let Some(audio) = self.view.audio.as_mut() else {
            // A press on a pill that is not drawn, which `audio_in` answers
            // `None` to and this cannot reach. Said rather than unreachable.
            return Acted::Nothing;
        };
        match ask {
            AudioAsk::Open => {
                audio.inputs = karakuri_environment::audio::inputs();
                let held = audio.inputs.len();
                println!(
                    "audio-in: `{}` — {}",
                    audio.word(),
                    match held {
                        0 => String::from(
                            "this machine has no audio inputs, and the card says so rather than                              opening empty"
                        ),
                        1 => String::from("one input to pick from"),
                        many => format!("{many} inputs to pick from"),
                    }
                );
                audio.opened();
                Acted::Nothing
            }
            AudioAsk::Shut => {
                audio.shut();
                Acted::Nothing
            }
            // **Out of this crate and into the one that can open a device.**
            // The pill names the input and `attached` opens it, which is the
            // seam ADR-0156 draws: a control asks, and whoever holds the
            // device decides.
            AudioAsk::Operation(operation) => {
                audio.shut();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// A press on the arrangement pill or on its menu, and what this program does
    /// about it.
    ///
    /// Five answers and this file decides none of them: which one a press asks for
    /// is `ArrangementPill::ask`'s, off the same laid-out pill `claim` hit-tested,
    /// and what arrives here is one of them by name. Two are moves of the control's
    /// own state and are this program telling the console about a press it cannot
    /// see; two are operations and go where every operation goes; the fifth is the
    /// reset, which is an `Op` and not a record, exactly as ADR-0208 has it —
    /// *"`ResetArrangement` reaches code; `RestoreArrangement` reaches a file"*.
    ///
    /// The menu shuts on anything that acts. An operator who has picked an item has
    /// finished with the list, and a card left standing over the console after the
    /// thing it was for has happened is the panel arguing with itself. It stays
    /// open for nothing, because nothing here can be picked twice.
    pub(crate) fn arranged(&mut self, ask: Ask) -> Acted {
        match ask {
            Ask::Open => {
                println!(
                    "arrangement: `{}` — save it, start a new one, or put one of {} back",
                    self.view.arrangement.word(),
                    self.view.arrangement.filed.len()
                );
                self.view.arrangement.opened();
                Acted::Nothing
            }
            Ask::Shut => {
                self.view.arrangement.shut();
                Acted::Nothing
            }
            // **The one flow on this panel that asks for letters.** Reached
            // only with no arrangement in use: with one in use, saving again
            // means that name and the pill asks for the operation instead.
            Ask::Name => {
                println!(
                    "arrangement: type a name and press return — letters, digits, `-` and \
                     `_`, and escape leaves it unsaved"
                );
                self.view.arrangement.asks_a_name();
                Acted::Nothing
            }
            // **The same operation `r` performs**, reached from the other end
            // of the panel exactly as the Outputs row's dot reaches `f`'s
            // fold. `Readout::op` is what says the arrangement in use is the
            // default again, whichever surface asked.
            Ask::Panel(op) => {
                self.view.arrangement.shut();
                Acted::Operated(self.op(op))
            }
            // **Down the path every other emitted operation takes**, which is
            // the whole reason `arrangement` sits on it: a record is written
            // by whoever holds the store, and the pill holds nothing.
            Ask::Operation(operation) => {
                self.view.arrangement.shut();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// What a press on a `uses` line did — the capsule, or a row of the card it
    /// puts down (`docs/adr/0329-…`).
    ///
    /// [`App::aimed`]'s shape one bay along and the same division: every arm is
    /// either this console's own state moving or one operation emitted down the
    /// path every other operation takes. Nothing is performed here — a `WireInput`
    /// is [`wired_input`]'s, which is the same re-aim a model's `wire_input`
    /// already goes through.
    ///
    /// A press on the capsule of a card that is down shuts it, because the capsule
    /// is *outside* the card and every press outside a card that is down is the
    /// dismissal. Pressing it twice therefore opens and closes, and no arm has to
    /// special-case it.
    pub(crate) fn wired(&mut self, ask: Wiring) -> Acted {
        match ask {
            Wiring::Chip { pane, node, input } => {
                let named = self
                    .view
                    .inspector
                    .get(pane)
                    .and_then(|pane| pane.nodes.get(node))
                    .and_then(|node| node.uses.get(input));
                match named {
                    Some(uses) if uses.candidates.is_empty() => {
                        println!(
                            "wire: `{}` takes one node of its kind and this deck holds only the                              one it is already wired to — there is nothing to pick",
                            uses.slot
                        );
                    }
                    Some(uses) => println!(
                        "wire: `{}` is wired to `{}` — pick a node to wire it to instead",
                        uses.slot, uses.to
                    ),
                    None => {}
                }
                self.view.open_wiring(pane, node, input);
                Acted::Nothing
            }
            Wiring::Shut => {
                self.view.shut_wiring();
                Acted::Nothing
            }
            // **The pick puts the card away, rewires, and emits**, which is one
            // gesture: the card left down over the pane the rebuild is about
            // would be a list to dismiss before the picture could be seen.
            //
            // **Emitted and performed where every other operation is**, which
            // is [`App::performed`]: the run's edge list moved to [`Engine`] so
            // that a press could reach it, because this type is the console's
            // readout and holds no engine at all. See [`wired_input`].
            Wiring::Pick(operation) => {
                self.view.shut_wiring();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// What a press on a pane head's `▾` did — the mark, or a row of the card it
    /// puts down (ADR-0338, decision 5).
    ///
    /// [`Readout::wired`]'s shape one row up and the same division: every arm is
    /// either this console's own state moving or one operation emitted down the
    /// path every other operation takes. Nothing is performed here — a `PointPane`
    /// is [`pointed_pane`]'s, which is where the pane is resolved and the deck
    /// refused.
    ///
    /// A press on the mark of a card that is down shuts it, for
    /// [`Readout::wired`]'s reason: the mark is outside the card and every press
    /// outside a card that is down is the dismissal.
    pub(crate) fn pointing(&mut self, ask: view::Pointing) -> Acted {
        match ask {
            view::Pointing::Mark(pane) => {
                println!(
                    "inspector: pane `{}` is showing deck {} — pick a deck to point it at",
                    view::PANE_NAMES.get(pane).copied().unwrap_or("?"),
                    deck_letter(self.view.pane_deck(pane)),
                );
                self.view.open_pane_target(pane);
                Acted::Nothing
            }
            view::Pointing::Shut => {
                self.view.shut_pane_target();
                Acted::Nothing
            }
            // **The pick puts the card away and emits**, and it is one
            // gesture: `View::point_pane` is what takes the card down, and it
            // is reached through [`pointed_pane`] so that a press and a
            // model's `operate` move the pointer by one route.
            view::Pointing::Pick(operation) => Acted::Emitted(Some(operation)),
        }
    }

    /// What a press on the Library bay's load control did — the button, the
    /// pulldown, or a row of the list it puts down (ADR-0305).
    ///
    /// [`Readout::arranged`]'s shape one bay along, and the two are the same
    /// division: every arm is either this console's own state moving or one
    /// operation emitted down the path every other operation takes. Nothing is
    /// performed here, and in particular nothing writes a file: a `LoadSet` is
    /// `played`'s, exactly as it is for the key and for the drop.
    ///
    /// A pick moves no deck selection, which is what the record is about:
    /// `View::aim_at` writes the bay's own mark and never `View::select`, so the
    /// ring on the strip and the letter in the foot are free to name two different
    /// decks. It also puts the list away, because a pick is one gesture and nothing
    /// here is emitted for a caller to end it in.
    pub(crate) fn aimed(&mut self, ask: Aim) -> Acted {
        match ask {
            Aim::Open => {
                println!(
                    "load: aimed at deck {} — pick a deck, or press `load` to send \
                     the cursor's Set there",
                    deck_letter(self.view.target_deck())
                );
                self.view.open_target();
                Acted::Nothing
            }
            Aim::Shut => {
                self.view.shut_target();
                Acted::Nothing
            }
            // **The whole of a pick**, and it asks for nothing: the mark is
            // the console's, exactly as the library cursor is, and no
            // operation in the vocabulary names it. `Operation::SelectDeck` is
            // emphatically not what this is — that one moves the keys.
            Aim::Deck(deck) => {
                self.view.aim_at(deck);
                Acted::Nothing
            }
            // **Down the path the key and the drop already take.** `played`
            // performs `LoadSet` by re-pointing the slot's source, so all
            // three routes arrive at the same place (ADR-0228).
            Aim::Load(operation) => Acted::Emitted(Some(operation)),
            // **A load with one operand missing is not a load**, and the
            // press says so rather than going quiet: P-0083, and the same
            // sentence `Released::Nowhere` is answered with one bay along.
            Aim::NoSet => {
                println!("load: nothing under the cursor — this library is listing no Sets");
                Acted::Nothing
            }
        }
    }

    /// A press on the Master bay's `+ add` or on its card, and what this program
    /// does about it.
    ///
    /// [`Readout::chosen`]'s shape one bay along, and the same division: the two
    /// answers that are the *console's* own state are performed here, and the one
    /// that is an operation leaves as one.
    ///
    /// The card is put away before the operation is emitted, which is
    /// [`Readout::chosen`]'s rule.
    pub(crate) fn chain_chose(&mut self, ask: view::Added) -> Acted {
        match ask {
            view::Added::Open => {
                let offers = self.view.chain_add.len();
                println!(
                    "chain: {offers} kind L5 procedure{} in this library — pick one to add it to \
                     the end of the master chain",
                    match offers {
                        1 => "",
                        _ => "s",
                    }
                );
                self.view.open_chain_add();
                Acted::Nothing
            }
            view::Added::Shut => {
                self.view.shut_chain_add();
                Acted::Nothing
            }
            view::Added::Add(operation) => {
                self.view.shut_chain_add();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// A press on the Sequencer bay's `+ lane`, and what this program does about
    /// it.
    ///
    /// [`Readout::aimed`]'s shape one bay along, and the same division: the two
    /// answers that are the *console's* own state are performed here, and the one
    /// that is an operation leaves as one. Nothing appends a lane here —
    /// `sequenced` is where `Operation::PointLane` lands, by the same road the
    /// bay's other four take.
    ///
    /// The card is put away before the operation is emitted, which is
    /// [`Readout::menued`]'s rule and its reason: a card left standing over a lane
    /// that has already been asked for would claim the next press on the console
    /// for a gesture the hand has finished.
    ///
    /// Opening it says what it is for, on `aimed`'s precedent: a card that went up
    /// in silence is a control an operator has to guess the shape of.
    pub(crate) fn chosen(&mut self, ask: Chose) -> Acted {
        match ask {
            Chose::Open => {
                let choices = self.view.lane_choices();
                println!(
                    "lane: {} target{} — {} fader{} and {} published control{} on deck {}",
                    choices.items.len(),
                    match choices.items.len() == 1 {
                        true => "",
                        false => "s",
                    },
                    choices.faders,
                    match choices.faders == 1 {
                        true => "",
                        false => "s",
                    },
                    choices.items.len() - choices.faders,
                    match choices.items.len() - choices.faders == 1 {
                        true => "",
                        false => "s",
                    },
                    deck_letter(self.view.target_deck())
                );
                self.view.open_lane();
                Acted::Nothing
            }
            Chose::Shut => {
                self.view.shut_lane();
                Acted::Nothing
            }
            Chose::Point(operation) => {
                self.view.shut_lane();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// A press on a row's menu, and what this program does about it.
    ///
    /// [`Readout::aimed`]'s shape one control along, and the same division: the two
    /// answers that are the *console's* own state are performed here, and the two
    /// that are operations leave as ones. Nothing is performed here, and in
    /// particular nothing writes a file: a `LoadSet` is `played`'s, exactly as it
    /// is for the button, the key and the drop, and a send is the window's, because
    /// a bundle is a disk read and a file written.
    ///
    /// The menu is put away before either operation is emitted, and it is put away
    /// on both arms rather than on one: a card left standing over a load that has
    /// already been asked for would claim the next press on the console for a
    /// gesture the hand has finished. That is `Menu::Naming`'s own rule at the
    /// arrangement pill, one bay along.
    ///
    /// The load moves no mark. `View::aim_at` is not called and neither is
    /// `View::select` or the cursor: the item named the deck and the row named the
    /// Set, so there is nothing left for this press to have moved — which is the
    /// whole of why the menu is a route worth having.
    pub(crate) fn menued(&mut self, ask: Picked) -> Acted {
        match ask {
            Picked::Open(row) => {
                println!(
                    "menu: `{}` — load it onto a deck, or save it as a kbset",
                    self.view
                        .sets()
                        .get(row)
                        .map(String::as_str)
                        .unwrap_or_default()
                );
                self.view.open_menu(row);
                Acted::Nothing
            }
            Picked::Shut => {
                self.view.shut_menu();
                Acted::Nothing
            }
            // **Down the path the button, the key and the drop already take.**
            // `played` performs `LoadSet` by re-pointing the slot's source, so
            // all four routes arrive at the same place (ADR-0228).
            Picked::Load(operation) => {
                self.view.shut_menu();
                Acted::Emitted(Some(operation))
            }
            // **The send leaves as an operation and the file is written where
            // every other disk write on this panel is** — the window, on the
            // branch a star and a keep already take, because a bundle is a
            // store read and a `.kbset` is a file (P-0091, ADR-0156).
            Picked::Send(operation) => {
                self.view.shut_menu();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// The name is finished, and what that asks for.
    ///
    /// One operation of the vocabulary, named — the same
    /// `Operation::SaveArrangement` the menu's *save* asks for with an arrangement
    /// already in use, so the two ways to reach a save are two ways to name one
    /// thing rather than two paths to a disk. The menu is shut before the operation
    /// is emitted, whether or not the name is any good: a name that is refused is
    /// refused out loud by `checked_name`, and a card left standing over the
    /// refusal would be the panel asking the question again without saying the
    /// answer.
    ///
    /// An empty name arrives here as an empty name and is refused there, which is
    /// the rule this file keeps everywhere: the surface owns the affordance and
    /// never the authority (P-0090).
    pub(crate) fn named(&mut self) -> Acted {
        let Some(typed) = self.view.arrangement.naming() else {
            return Acted::Nothing;
        };
        let name = typed.to_owned();
        self.view.arrangement.shut();
        Acted::Emitted(Some(Operation::SaveArrangement { name }))
    }

    /// Commits whichever inline text input session is active, emitting the save operation.
    pub(crate) fn commit_active_text_input(&mut self) -> Acted {
        match self.view.active_text_input() {
            Some(view::TextInputKind::Arrangement) => self.named(),
            Some(view::TextInputKind::DeckName(_)) => Acted::Emitted(self.view.named_set()),
            None => Acted::Nothing,
        }
    }

    /// A press on the Program bay head's `solo`. The pill says what it did — which
    /// of the two operations it asked for — because the whole point of the control
    /// is that it is the same solo `s` and `u` perform, reached from a capsule
    /// instead of from the pointer.
    ///
    /// The region is the picture's and never the pointer's, which is the one way
    /// this differs from `s`: a key solos whatever the pointer is over, and this
    /// pill names `program-view` because `docs/manual/console.html` says what it is
    /// for — *"Solo the program view: the panel folds away and only the picture is
    /// left, which is also how you capture this window."* A press on the grip in a
    /// bay head. It says what it did, because the point of the control is that it
    /// is the fold `f` performs, reached from the console's own shape instead of
    /// from the keyboard. One operation and no toggle: a folded bay has no
    /// rectangle, so the grip is not drawn afterwards and the way back is `z`.
    ///
    /// One control and not two. ADR-0295 gave a pane a band on its outer edge and
    /// this doc described both; ADR-0300 replaced that half — a pane folds by its
    /// own boundary being pulled past the narrowest it goes, and comes back by that
    /// boundary being dragged in, so it needs no press arm and its way back is not
    /// `z` alone.
    pub(crate) fn folded(&mut self, op: Op) -> Outcome {
        let Op::Fold(id) = op else {
            unreachable!("a fold control asked for {op:?}")
        };
        println!("fold: {} folds away — `z` brings it back", self.label(id));
        self.op(op)
    }

    pub(crate) fn soloed(&mut self, op: Op) -> Outcome {
        println!(
            "program: {}",
            match op {
                Op::Solo(_) =>
                    "solo the picture — everything else folds away, and the window                      is that region",
                Op::Unsolo =>
                    "the solo comes off — what was folded before it comes back,                      including whatever was already folded",
                other => unreachable!("the solo pill asked for {other:?}"),
            }
        );
        self.op(op)
    }

    /// A press on one of the four class pills, and the one press in this program
    /// that is neither an operation on the arrangement nor one on the mix.
    ///
    /// # Why it takes a different path from every other press in this file
    ///
    /// Everything else here ends in one of two places. A control over the console's
    /// own shape asks for a [`Op`], `Panel` performs it, and what comes back is an
    /// [`Outcome`]. A control over the mix emits an [`Operation`], [`written`]
    /// turns it into a `Record` and [`apply`] moves the deck with it — P-0090, and
    /// every control ends at the same record. A reader who has just met those two
    /// will reach for the second here, because it is the one every new control has
    /// taken for a year.
    ///
    /// It must not be routed as an `Operation`, and
    /// [ADR-0236](../../../docs/adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md)
    /// is explicit about it. The opening is configuration of the *map* — the layer
    /// every surface reaches the vocabulary through — and not a member of the
    /// vocabulary the map addresses. The rule is narrower than *map configuration
    /// is never an operation*, because `Operation::PointLane` already is one: a
    /// setting that decides whether a surface may reach a class of operations
    /// cannot itself be one of those operations. Rule 01 would make such an
    /// operation reachable from all four surfaces, MCP included, and a permission
    /// an actor can grant itself is not a permission. There is no 65th row on the
    /// operations page for the same reason, and ADR-0235's *"the opening setting
    /// has no operation"* is annotated as settled by exactly this.
    ///
    /// So: no `Operation`, no `Record`, no [`Acted::Emitted`]. What a press hands
    /// over is a value — `McpPill::next`, the opening with one class set the other
    /// way and the other three written back as they were — and the run's `Opening`
    /// is where it goes. Somebody will one day try to fix this into the vocabulary;
    /// this paragraph is what it costs them to do it, and `Acted::Opened` is the
    /// type that will not let it happen quietly.
    ///
    /// The view's copy is written in the same breath as the handle, not left for
    /// the next frame's read. `input::claim` and the probe above both hit-test
    /// against `View::opening`, and the pill is not the same width in its two
    /// states — so a press that moved the handle and not the view would leave the
    /// very next press aimed at the capsule that was there before it.
    pub(crate) fn opened(&mut self, pill: &McpPill) -> Acted {
        // **Annotated**: it says what a press composes — an opening and not a `bool`.
        let next: Open = pill.next(self.view.opening);
        self.opening.set(next);
        self.view.opening = next;
        let open = next.holds(pill.class);
        // **What the pill says it did, in the words a refusal says it in.**
        // `Class::title` and `Class::opened_at` are the gate's own strings, so
        // the sentence a model is refused with and the sentence an operator
        // reads at the pill name one thing the same way (P-0090).
        println!(
            "{}: `{}` — {} is {} to a model. {}. the operator opens it at {}.",
            pill.class.bay(),
            view::mcp_word(open),
            pill.class.title(),
            match open {
                true => "open",
                false => "shut",
            },
            match open {
                true => "calls in this class are performed",
                false => "calls in this class are refused, and the refusal says so",
            },
            pill.class.opened_at()
        );
        Acted::Opened
    }

    /// A press on a slot MCP policy pill, cycling Auto -> On -> Off -> Auto.
    pub(crate) fn cycle_slot_policy(&mut self, deck: usize) -> Acted {
        if deck < karakuri_console::view::DECKS {
            let next = self.view.slot_policies[deck].next();
            self.view.slot_policies[deck] = next;
            self.slot_policies.set_policy(deck, next);
            println!(
                "slot {}: MCP policy set to {:?} (`{}`)",
                deck + 1,
                next,
                next.pill_word()
            );
        }
        Acted::Opened
    }

    /// A press on a scope chip, and it is the surface performing its own pointer —
    /// [`pointed`]'s shape one bay along, done here rather than in `performed` for
    /// the reason the scope key's is done at the key.
    ///
    /// `Operation::SelectScope`'s payload is `Undecided`, so a performer reading
    /// the operation could not tell which library was chosen and would have to
    /// guess. The press *knows*, because a pointer lands on one capsule and no
    /// other, and [`Chosen`] is what carries the two halves together. So the mark
    /// is moved here and the operation is emitted for the record it is owed, which
    /// is `Silent(Surface)` — the same shape as the key, which steps first and
    /// emits afterwards.
    ///
    /// A chip that is already marked is not refused, and the line says which of the
    /// two it was. `View::select_scope` answers `false` for it, and that is a mark
    /// that did not move rather than a press that failed: where the key *steps* and
    /// would go somewhere else, a press names, and naming the library you are
    /// already reading is asking it again. What the caller does with that is
    /// re-read the listing, which is where a directory read belongs (P-0091) and is
    /// not on this side of the seam.
    ///
    /// It cannot refuse for the other reason either: the chip came out of
    /// `View::scopes`, so it is on the row by construction.
    pub(crate) fn chose(&mut self, chosen: Chosen) -> Acted {
        let moved = self.view.select_scope(chosen.scope);
        println!(
            "scope: `{}` — {}",
            chosen.scope.name(),
            match moved {
                true => "the library this bay reads, and the cursor is back at the top of it",
                false => "already the library this bay reads, so this asks that one again",
            }
        );
        // **The Set the walk is of is read on the way out**, and it is this
        // side's answer rather than the chip's: `Operation::WalkHistory` names
        // a Set, the console holds a deck letter, and the id rides the aim
        // (ADR-0308). `View::aimed` is where this file writes it, per frame
        // beside every other reading, and `Chosen::asked` is the one place it
        // is read — so the operation this press emits and the listing the press
        // below re-reads are narrowed by one value. The four library chips
        // ignore it and emit `SelectScope`, which carries nothing.
        Acted::Emitted(Some(chosen.asked(self.view.aimed.as_deref())))
    }

    /// A press on one of the Library bay's two filter fields, and it is
    /// [`Readout::chose`]'s shape one row down: the surface performs its own
    /// pointer and emits the operation for the record it is owed, which is
    /// `Silent(Question)` — *it asks rather than changes*.
    ///
    /// The operation carries everything, where `SelectScope` carries nothing.
    /// `Operation::ListSets { holds, layer }` is exactly the state the two fields
    /// are in, so this applies it rather than guessing at it and there is no value
    /// travelling beside it. `View::narrow` is the one door into that state and is
    /// where a `holds` this console cannot draw is refused — which nothing here can
    /// hand it, because the value came out of `LibraryBay::filter` stepping the
    /// same candidates.
    ///
    /// The listing is not read here. It is a directory read
    /// ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md))
    /// and the store is the window's rather than the readout's, so the caller
    /// re-reads on `Operation::ListSets` exactly as it does on
    /// `Operation::SelectScope` — one branch, two operations, because a scope and a
    /// filter are the same question asked of different halves.
    ///
    /// A filter set while the bay is reading something else is said out loud, and
    /// it is the one thing about this row that would otherwise be silent: the
    /// operation is *List what the store holds*, which is `all` and the `my sets`
    /// starred out of it, and `presets` and `folder` are not the store. The press
    /// is still a real question — it is answered the moment one of those two is
    /// marked again — and a press that appears to do nothing is what this line
    /// exists to prevent.
    pub(crate) fn narrowed(&mut self, operation: Operation) -> Acted {
        let holds = match &operation {
            Operation::ListSets { holds, .. } => holds.as_deref(),
            // **A kind press keeps the field where it is**, which is the whole
            // of what two controls on one row means: `FilterLibrary` carries
            // the six chips and says nothing about `holds`, so the value that
            // goes back into `View::narrow` is the one the field is already on.
            Operation::FilterLibrary { .. } => self.view.filters().holds,
            _ => {
                unreachable!("the filter row emits `ListSets` and `FilterLibrary` and nothing else")
            }
        };
        let holds = holds.map(str::to_owned);
        let kinds = match &operation {
            Operation::FilterLibrary { kinds } => *kinds,
            // **And a `holds` press keeps the chips where they are**, for the
            // reason above read the other way: `ListSets` carries no kinds.
            _ => self.view.filters().kinds,
        };
        let moved = self.view.narrow(holds.as_deref(), kinds);
        let at = self.view.filters();
        println!(
            "filter: `{}` / {} — {}",
            at.holds_word(),
            showing(at.kinds),
            match (self.view.scope(), moved) {
                (Some(Scope::AllSets | Scope::MySets), true) =>
                    "the listing under it is what the store holds, narrowed, and the cursor is \
                     back at the top of it",
                // **A step that arrived where it already was**, which is the
                // `holds` field on a store whose Sets name no node: the press
                // asks for the listing again, and that is `Readout::chose`'s
                // answer for the chip that is already marked.
                (Some(Scope::AllSets | Scope::MySets), false) =>
                    "already what this bay is narrowed to, so this asks the store that same \
                     question again",
                _ =>
                    "this narrows the store's own listing, which is `all` and the `my sets` \
                      starred out of it — neither is the library this bay is reading, so mark \
                      one of them and the rows follow",
            }
        );
        Acted::Emitted(Some(operation))
    }

    /// A press on the Library bay's `params` chip, and it is
    /// [`Readout::narrowed`]'s shape one row down with one difference: only one of
    /// the two things a press on this chip can mean is an operation.
    ///
    /// Opening asks for a reading and this file does not answer it, which is
    /// [`Readout::narrowed`]'s division exactly: what a Set declares is on a disk,
    /// the store is the window's rather than the readout's, and a press is where
    /// this program already reads one. So the operation leaves here and the caller
    /// answers it with [`read_reading`], on the same branch it re-reads a listing
    /// on.
    ///
    /// Closing is performed here and emits nothing. It changes which rows this bay
    /// is drawing, which is the console's own state — no more an operation than a
    /// fold is — and a `ReadSet` emitted to put a reading away would say a question
    /// was asked at the moment one stopped being. See `view::Read`, where the
    /// argument is.
    pub(crate) fn asked_to_read(&mut self, ask: Read) -> Acted {
        match ask {
            Read::Open(operation) => Acted::Emitted(Some(operation)),
            Read::Shut => {
                println!(
                    "read: closed — {}",
                    match self.view.shut_reading() {
                        true => "the list is a list again, and the chip asks for it back",
                        // The chip answers `Shut` off the block the bay is
                        // drawing, so this is a reading that went away between
                        // the layout and the press. Said rather than
                        // unreachable.
                        false => "there was nothing open",
                    }
                );
                Acted::Nothing
            }
        }
    }

    /// A press on a row of the Library bay's list, which takes that Set in hand and
    /// asks for nothing.
    ///
    /// # The mark on the row is the whole of what a carry can draw
    ///
    /// `docs/manual/console.html` draws no drag affordance and no drop target — no
    /// ghost under the pointer, no lit strip — and this program draws what that
    /// page draws. What it *does* draw is `.lib-row.cursor`, and the row a hand is
    /// on is exactly what that mark is for, so the press moves it: the row taken is
    /// the row marked, for the length of the carry and afterwards.
    ///
    /// Afterwards is deliberate. The cursor is the operand a load reads
    /// (`view::View::cursor_row`), so a drop that landed and a carry that was let
    /// go over nothing both leave the keyboard aimed at the Set the hand last
    /// touched — *two ways in, one name*, met at the pointer this bay keeps rather
    /// than only at the operation.
    ///
    /// It emits nothing, and there is nothing for it to emit. Moving this cursor
    /// has no row on `docs/manual/operations.html` and is not owed one, and
    /// `docs/manual/console.html` is where that is said: *"The cursor moves on the
    /// arrow keys and gets no row on the operations page, which is a decision and
    /// not an omission"* — a pointer that names a row instead of stepping to it is
    /// the same pointer, which is what `view::View::point_at` is.
    ///
    /// And a pointer that is the same pointer owes what the keys owe.
    /// `view::View::opened` draws the reading only where the row under the cursor
    /// is still the Set it was read of, and the rule that keeps that honest is the
    /// cursor's rather than the keyboard's: *"the reading follows the cursor: a
    /// move with one open is a read of the row it arrived at"*
    /// (`karakuri-console/src/view.rs`, `view::View::reading_open`). So this
    /// answers [`Acted::Pointed`] where the mark actually moved, exactly as the
    /// arrow keys answer `Change::Pointed(moved)`, and the window loop re-reads on
    /// it the way it re-reads on theirs. A carry that discarded the `bool` made the
    /// block under an open reading vanish for the length of the run, because
    /// nothing else on this route ever moves the cursor back.
    ///
    /// That is still no operation. `read_reading` reads the store and writes the
    /// answer into the view; it emits nothing, so the press
    /// `karakuri_console::input`'s own doc calls *"the one offer on this console
    /// whose press names no operation"* goes on naming none (ADR-0265).
    ///
    /// That sentence is now qualified rather than untrue: it is about the four
    /// scopes whose rows are Sets. Under `history` the same rectangle is
    /// [`Readout::landed`], which names `Operation::RestoreProcedure` outright —
    /// and this method is not reached there, because `View::sets` hands the carry
    /// nothing (ADR-0308).
    pub(crate) fn took(&mut self, p: Point, taken: Taken) -> Acted {
        let Taken {
            row,
            set,
            procedure,
        } = taken;
        // **The mark first, and the hand after it.** Both are this console's
        // own pointers and neither is an operation, so the order is only about
        // the borrow — but the mark is what says the press was seen.
        let moved = self.view.point_at(row);
        println!(
            "press ({:.0}, {:.0}): `{set}` is in hand — let it go over a strip to load it there, \
             or anywhere else to load nothing",
            p.x, p.y
        );
        self.panel.carry(p, set, procedure);
        // **The `bool` is answered rather than dropped**, which is the whole
        // of the re-read above: a row the hand arrived at is a row the reading
        // moves to, and a press that landed on the row the cursor was already
        // on moved nothing and asks for nothing.
        match moved {
            true => Acted::Pointed,
            false => Acted::Nothing,
        }
    }

    /// A press on a row of the Library bay's `history` scope, which lands that
    /// version on the node it was a version of.
    ///
    /// [`Readout::took`]'s neighbour on the same rectangle, and the two are the
    /// same press meaning two things: a row of a library is a Set to take in hand
    /// and a row of a history is a version to put back. Which of them answers is
    /// decided by which listing went in with the point (`view::View::sets`,
    /// `view::View::versions`) rather than by an arm here asking the scope.
    ///
    /// It emits and performs nothing, which is [`Readout::asked_to_read`]'s
    /// division: what a landing does is write a file the store owns, and the store
    /// is the window's rather than the readout's. [`restored`] is where it is done,
    /// on the branch every emitted operation already takes.
    ///
    /// The cursor is not moved. A carry moves it because the mark on a row is what
    /// a drag has to draw and because a load reads it afterwards; a landing reads
    /// neither — the row is the operand and the deck is the pulldown's — so moving
    /// the mark would be this press quietly re-aiming the key beside it.
    pub(crate) fn landed(&mut self, operation: Operation) -> Acted {
        println!(
            "press: {} — the version is written over that node's working copy and its \
             watcher builds it, judged against the budget like any edit",
            match &operation {
                Operation::RestoreProcedure {
                    deck,
                    revision: karakuri_operation::Revision::Picked(version),
                } => format!("`{version}` put back on deck {}", deck_letter(*deck)),
                // **The staging lane's arm, which names a node rather than a
                // version.** Which version that is, is `restored`'s to work
                // out — the one before the one running, out of the store's
                // history — so this says the node and lets the performance
                // say the file (ADR-0326).
                Operation::RestoreProcedure {
                    deck,
                    revision: karakuri_operation::Revision::Previous(node),
                } => format!(
                    "{} on deck {} stepped back one version",
                    node_addr(ir_layer(node.layer), node.index),
                    deck_letter(*deck)
                ),
                other => format!("{other:?}"),
            }
        );
        Acted::Emitted(Some(operation))
    }

    /// A press on a candidate row, which keeps that candidate.
    ///
    /// [`Readout::landed`]'s neighbour on the same row, and the two are what the
    /// lane offers: the capsule steps a node back a version and the row around it
    /// says *I have looked at this*. The free act is on the large target and the
    /// act that writes a file is on the small one, which is the whole of why they
    /// are arranged this way round (ADR-0326).
    ///
    /// It emits and performs nothing here, which is [`Readout::asked_to_read`]'s
    /// division: what a keep changes is the lane, and the lane is `View::staging`,
    /// which this readout owns but the window writes — so [`kept`] is where the row
    /// is taken off, on the branch every emitted operation already takes. `written`
    /// answers `Silent(Silent::Surface)` for it, which is that division said in the
    /// record vocabulary.
    pub(crate) fn kept(&mut self, operation: Operation) -> Acted {
        println!(
            "press: {} — it settles the node and leaves the lane; the picture does not move \
             and no record is written",
            match &operation {
                Operation::KeepCandidate { deck, node } => format!(
                    "{} kept on deck {}",
                    node_addr(ir_layer(node.layer), node.index),
                    deck_letter(*deck)
                ),
                other => format!("{other:?}"),
            }
        );
        Acted::Emitted(Some(operation))
    }

    /// A press on the Outputs row's one control. The dot says what it did — which
    /// of the two operations it asked for, and what the picture is now — because
    /// the whole point of the control is that it is the same fold `f` over the
    /// picture performs, reached from the other end of the panel.
    pub(crate) fn sink(&mut self, asked: Operation, op: Op) -> Outcome {
        // Two operations and no third, which is `Outputs::op`'s whole
        // argument: the toggle is the dot choosing between them, and what
        // arrives here is one of the two by name.
        // **What was asked and what performs it, in one line.** The
        // operation names the output — `RouteFrame { output: Program, on }` —
        // and the fold is how this surface carries it out, which is one fact
        // said once rather than a second stored `on` beside the layout node.
        println!(
            "outputs: {} — {}",
            match asked {
                Operation::RouteFrame { output, on } => format!(
                    "{} {}",
                    output.name(),
                    match on {
                        true => "on",
                        false => "off",
                    }
                ),
                ref other => format!("{other:?}"),
            },
            match op {
                Op::Fold(_) =>
                    "the sink was on, so the picture folds away and the \
                                inspector takes its height",
                Op::Unfold(_) =>
                    "the sink was off, so the picture comes back — with \
                                  whatever was folded over it",
                other => unreachable!("the dot asked for {other:?}"),
            }
        );
        self.op(op)
    }
}

/// A pointer event, stripped to what the rule needs.
///
/// A button is left, or it is the secondary button, or it is not routed at all.
/// This said *a button is left or it is not routed* until 2026-09-09, and it
/// was exact: `window_event` matched `MouseButton::Left` and every other button
/// fell through its `_ => {}`, so a right press reached nothing in this program
/// and `egui` was never told about one either.
///
/// What made it a second variant rather than a field on [`Pointer::Down`] is
/// where the answer is wanted. `karakuri_console::input::claim` decides *whose*
/// an event is from where the pointer is and has never known which button a
/// press was — rules 1 to 4 are all about position — and nothing about that
/// changes: a secondary press on a control the console draws is the console's
/// for the same reason a primary one is. What differs is only what the press
/// then *asks* for, which is this file's half of the seam. A `Down { secondary:
/// bool }` would have put the flag through every arm of the press handler to be
/// read by one of them
/// ([ADR-0311](../../../docs/adr/0311-a-row-menu-loads-a-set-onto-a-named-deck-and-saves-it-through-the-systems-own-dialog.md)).
///
/// There is no `Secondary` release, and that is the whole of what this button
/// does here: a secondary press opens a menu and the gesture ends at the *next*
/// press, which is [`Pointer::Down`]'s or this one's again. Nothing is taken in
/// hand on a secondary press, so there is nothing for a release to let go of.
///
/// The wheel carries a distance now, and only one axis of it. It used to carry
/// nothing, because nothing on this panel did anything with one — *which wheel
/// axis it was does not change who gets it* is what this said, and it was true
/// while the answer was always `egui`'s. An Inspector pane scrolls down its
/// list
/// ([ADR-0307](../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)),
/// so the vertical distance is now the whole of what the event says; a
/// horizontal one reaches nothing here and is dropped where the two are pulled
/// apart, in `window_event`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Pointer {
    Moved(Point),
    Down,
    Up,
    /// A press of the secondary button, which on this panel opens the menu on a row
    /// of the Library bay's list and does nothing anywhere else.
    Secondary,
    /// How far to scroll, in logical pixels, positive down the list — a notch of a
    /// mouse wheel converted to `karakuri_console::room::size::WHEEL_STEP` and a
    /// trackpad's own pixels passed straight through.
    Wheel(f32),
}

/// What routing a pointer event did, beyond deciding whose it was.
///
/// A list rather than an `Option<Outcome>`, because the controls on this panel
/// end in more than one place: the Outputs dot asks for an operation on the
/// *arrangement*, which this crate performs and reports as an [`Outcome`], a
/// fader asks for an operation on the *mix*, which nothing in
/// `karakuri-console` can perform at all, and two of them ask for no operation
/// and are still not nothing. The repaint decision is taken from which of them
/// it is — see `Change::Operated` and `Change::Emitted`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Acted {
    /// Nothing acted: a press on a boundary, a move, a wheel, a release.
    Nothing,
    /// The Outputs dot, and what the operation it named did.
    Operated(Outcome),
    /// A fader translated a drag into the vocabulary, or the drag moved the pointer
    /// over a value that did not change and asked for nothing.
    Emitted(Option<Operation>),
    /// A class pill was pressed, and what it wrote went to the run's `Opening`
    /// rather than to the arrangement or to the deck.
    ///
    /// A fourth answer rather than a reuse of `Nothing`, and the difference is the
    /// whole of ADR-0236: this press is not an operation and must never be made
    /// into one, so it cannot be an `Emitted`; and it is not nothing either,
    /// because a word on the panel changed. See `Readout::opened`.
    ///
    /// It carries no payload because there is none to carry: what changed is held
    /// in the `Opening`, which is a handle another surface reads, and a copy of it
    /// in this enum would be the second answer to *what is open*.
    Opened,
    /// A press moved the library cursor, and asked for nothing ([`Readout::took`]).
    ///
    /// It is the console's one pointer a press moves *without* naming an operation:
    /// the deck selection moves on a press too, and reaches [`Acted::Emitted`]
    /// through `Operation::SelectDeck` and [`pointed`], which is
    /// `Change::Pointed`'s note on the same pair.
    ///
    /// A fifth answer rather than a reuse of [`Acted::Nothing`], which is
    /// [`Acted::Opened`]'s argument one control along: this press names no
    /// operation and must not be made into one (ADR-0265), so it cannot be an
    /// [`Acted::Emitted`]; and it is not nothing either, because the reading
    /// follows the cursor — a move with one open is a read of the row it arrived at
    /// (`karakuri_console::view::View::reading_open`), and a caller that could not
    /// tell this press from a boundary's would leave the block drawn nowhere.
    ///
    /// The caller is what owes that read, and not [`Readout::took`]: a reading is a
    /// file, and the store is the window's rather than the readout's — the division
    /// [`Readout::asked_to_read`] is written to.
    ///
    /// It carries no payload because there is none to carry: it is answered only
    /// where the cursor moved, which is `View::point_at`'s `bool`, and a copy of
    /// the row here would be a second answer to `View::cursor_row`.
    Pointed,
}
