//! Pointer event translation and bay event dispatch methods.

use karakuri_console::egui;
pub(crate) use karakuri_operation::gate::{Class, Open};
use karakuri_operation::Output;

use super::*;

pub(crate) mod actions;
pub(crate) mod press;
pub(crate) mod types;
pub(crate) use types::*;

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
        if matches!(event, Pointer::Down)
            && !self.panel.dragging()
            && !self.view.has_modal_overlay()
        {
            if let Some(bay) = karakuri_console::focus::bay_at(self.panel.layout(), at) {
                self.view.focus_bay(&self.panel, bay.name);
            }
        }
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
}
