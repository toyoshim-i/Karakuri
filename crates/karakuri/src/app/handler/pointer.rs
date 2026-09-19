//! Pointer and mouse event dispatch for console and bays.

use karakuri_console::input::Claim;
use karakuri_console::repaint::Change;
use karakuri_layout::Point;
use karakuri_operation::SetTransfer;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;

use super::super::*;
use crate::app::operations::routed;

impl App {
    pub(crate) fn handle_pointer_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: WindowEvent,
    ) {
        let Some(gfx) = self.gfx.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let p = Point::new(
                    (position.x / self.scale) as f32,
                    (position.y / self.scale) as f32,
                );
                let ctx = gfx.egui.egui_ctx().clone();
                let (claim, acted) = self.readout.pointer(&ctx, Pointer::Moved(p));
                if claim == Claim::Egui {
                    App::to_egui(gfx, &mut self.costs, &event);
                }
                // **A move can now change the mix**, which no pointer event
                // could before: a fader in hand turns this move into one
                // operation of the vocabulary. What is owed for it is what the
                // drag asked for and not the claim — a fader held against the
                // top of its track asks for 1.0 sixty times a second and
                // changes nothing, and `Change::Pointer(Panel)` would draw a
                // frame for every one of them.
                let repaint = App::performed(
                    gfx,
                    self.started,
                    &mut self.readout,
                    self.recording.recorder(),
                    &acted,
                    Change::Pointer(claim).repaint(),
                )
                .repaint;
                // **And the hover layer is told where the pointer went**, with
                // the claim `input::claim` has just answered: `Claim::Egui` is
                // the panel saying the pointer is on none of its controls, so
                // the common move costs one comparison there. What comes back
                // is a frame owed **now** only where a tip is on screen that
                // must not be — the dwell itself is a deadline and is asked
                // for on the frame, beside `View::animating`.
                let tip = self.hover.moved(
                    claim,
                    &self.readout.panel,
                    &ctx,
                    &self.readout.view,
                    p,
                    self.started.elapsed(),
                );
                let repaint = repaint.soonest(Change::Tip(tip).repaint());
                App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
            }
            // **The pointer left the window**, which is not a move to
            // anywhere: a tip that is up goes, and no dwell is running. Only
            // the hover layer cares — `egui` is told either way, because this
            // event is not one `input::claim` has a rule about.
            WindowEvent::CursorLeft { .. } => {
                App::to_egui(gfx, &mut self.costs, &event);
                let tip = self.hover.left();
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Tip(tip).repaint(),
                );
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                let which = match state {
                    ElementState::Pressed => Pointer::Down,
                    ElementState::Released => Pointer::Up,
                };
                let ctx = gfx.egui.egui_ctx().clone();
                let (claim, acted) = self.readout.pointer(&ctx, which);
                if claim == Claim::Egui {
                    App::to_egui(gfx, &mut self.costs, &event);
                }
                // **A press that named a scope is a listing to read**, and
                // it is read here because this is where the store is: a scope
                // *is* a listing on this side, and a directory read is not a
                // thing to do on a frame (P-0091). It is read on **every**
                // chip press and not only on one that moved the mark, which is
                // the one place this parts company with `e`: the key steps and
                // so a press that changed nothing asked for nothing, where a
                // pointer *names* — and naming the library you are already
                // reading is asking it again, which is a question this console
                // had no way to put before.
                // **And a press that narrowed one is the same question asked
                // of the other half**, so it is the same branch: a scope and a
                // filter both change what the store is being asked, and the
                // answer to either is a directory read this side owns.
                // **And a press on a star is a file beside the Sets to
                // write**, taken before the re-read below because the listing
                // it re-reads is the one this write changes: `my sets` is the
                // starred subset (ADR-0299), so a star taken off under that
                // chip is a row that leaves. See [`favourite`], which answers
                // `None` for every other operation and writes nothing else.
                // **A chip in the Outputs row that is not the picture**, and
                // it is taken here because this is where the event loop is:
                // `winit` will not make a window without one, and
                // `App::performed` below is handed a `Gfx` and no loop. See
                // [`routed`], which is the whole of what a projector costs
                // this file.
                if let Acted::Emitted(Some(Operation::RouteFrame { output, on })) = acted {
                    if let Some(line) = routed(gfx, event_loop, output, on) {
                        println!("{line}");
                    }
                }
                if let Acted::Emitted(Some(ref operation @ Operation::SetFavourite { .. })) = acted
                {
                    // **`Asked::Operator`, because a hand on this panel is the
                    // operator's own act** — the same word `k` and the `keep`
                    // capsule pass for a save, and what it decides is
                    // `favourite`'s own (P-0096, ADR-0301).
                    if let Some(line) = favourite(&self.store, Asked::Operator, operation) {
                        println!("{line}");
                    }
                }
                // **A star is on this branch as well as the two above**, and
                // it is the same question for the same reason: what the bay
                // draws is a listing and a set of marks, both of them read off
                // a disk, and the write above changed one of them.
                // **And a press on the `history` chip is a walk of the store**,
                // which is the same branch because it is the same act: the
                // fifth chip marks a scope like the four beside it and asks for
                // a different row (ADR-0308), so a press that emitted
                // `WalkHistory` and did not re-read left the bay drawing the
                // listing it had before under a mark that says `history`. It
                // was missing here until 2026-09-10 and is `docs/adr/0342-…`'s
                // own defect.
                if matches!(
                    acted,
                    Acted::Emitted(Some(
                        Operation::SelectScope { .. }
                            | Operation::WalkHistory { .. }
                            | Operation::ListSets { .. }
                            | Operation::SetFavourite { .. }
                    ))
                ) {
                    // **Whose history, read before the listing is rewritten**:
                    // a scope press can be the one that marks `history`, and
                    // what that scope lists is the Set the load pulldown's deck
                    // is running (`aimed_set`).
                    let running = aimed_set(gfx, &self.readout.view);
                    println!(
                        "{}",
                        listing(
                            &mut self.readout.view,
                            &self.store,
                            self.presets.as_ref(),
                            self.folder.as_deref(),
                            running.as_deref(),
                        )
                    );
                }
                // Post-event side-effects (Set reading and disk saving) coordinated through operations.rs.
                handle_post_event_side_effects(
                    &mut self.keeping,
                    &gfx.engine,
                    &self.store,
                    &mut self.readout.view,
                    &acted,
                );
                // **And a press that asked to keep a node's procedure is one
                // file to write**, here for the Set keep's reason one line up:
                // the bytes are the run's, the store is the window's, and a
                // disk write is not a thing to do on a frame (P-0091).
                //
                // **`Asked::Operator`, so it lands in `<store>/procedures/`**
                // — a hand on this panel is the operator's own act, which is
                // what makes that tier exist (P-0096). A model's arrives
                // through the operate drain and carries `Asked::Model`.
                //
                // **The id is the pane head's if one is being typed there.**
                // The capsule emits `None`, because it is the press that types
                // nothing (ADR-0128); the head three items along is this
                // console's second letter-taking flow, and a keep sent while
                // that head is asking files under what was typed. The head is
                // looked up by the deck the operation names rather than by the
                // pane the capsule was drawn in, for `View::named_set`'s own
                // reason: the gesture spans frames and the deck is read at the
                // commit.
                if let Acted::Emitted(Some(Operation::KeepProcedure { deck, node, ref id })) = acted
                {
                    let id = id.clone().or_else(|| self.readout.view.naming_over(deck));
                    self.keeping.keep_procedure(
                        &gfx.engine,
                        &self.store,
                        Asked::Operator,
                        usize::from(deck),
                        node,
                        id,
                        None,
                    );
                }
                // **And a press that asked to send a Set is a dialog to open
                // and a file to write**, here for the reason the three above
                // it are: the store is the window's, a bundle is a store read
                // and a file written, and neither is a thing to do on a frame
                // (P-0091, ADR-0156). What this side adds to the operation is
                // the destination, which the operation deliberately does not
                // carry (ADR-0260): a read's answer goes where the surface
                // that asked puts answers, and this surface asks the platform.
                //
                // **Both buttons reach this line**, because the item is picked
                // by whichever press lands on the card while it is down — see
                // the `Secondary` arm below, which calls the same function.
                if let Acted::Emitted(Some(Operation::TransferSet {
                    transfer: SetTransfer::Send { ref id },
                })) = acted
                {
                    println!("  send: naming a file to write `{id}` to");
                    sending(
                        &gfx.window,
                        &self.store,
                        self.folder.as_deref(),
                        id,
                        self.keeping.send_tx.clone(),
                    );
                }
                // **And a press on the `rec` pill is a recording started or
                // stopped**, here for the reason the three above it are: the
                // engine and the store are the window's, and neither end of
                // this is a thing to do on a frame (P-0091) — a start writes
                // the Set file a replay reconstructs the session from, and a
                // stop blocks on the writer thread. The reading is taken here
                // and every byte of I/O is on a thread of its own; see
                // [`Sessions`].
                if let Acted::Emitted(Some(Operation::RecordSession { ref recording })) = acted {
                    self.recording
                        .asked(&self.keeping, &gfx.engine, &self.store, recording);
                }
                // **And a press that moved the library cursor owes that same
                // read** (ADR-0265): [`reread_if_open`] is the arrow keys'
                // own call one event along, and it is here for the reason the
                // two calls above it are — the store is the window's, a file
                // read is not a thing to do on a frame (P-0091), and
                // `karakuri-console` reaches no disk at all (ADR-0156).
                //
                // **The press still names no operation.** `read_reading`
                // emits none, and `Acted::Pointed` is not an `Acted::Emitted`.
                if let Some(line) = reread_if_open(
                    matches!(acted, Acted::Pointed),
                    &mut self.readout.view,
                    &self.store,
                ) {
                    println!("{line}");
                }
                // **A Set dropped out of `presets` is taken in before it is
                // loaded**, which is the load's two-moment press arriving at the
                // pointer: a preset row names a file and a load names an id,
                // and taking it in is what gives the Set the id the load needs
                // (ADR-0229). Two routes to one row have to reach the same
                // place — `console.html`'s *two ways in, one name* — and a
                // drop that skipped this would name a Set this store does not
                // hold and be refused where the key succeeds.
                //
                // **Here rather than in the console**, for the reason every
                // other store question is here: `karakuri-console` reaches no
                // disk at all (ADR-0156), so the drop names the row it was
                // dragged from and this side turns that into the two rows of
                // the vocabulary one gesture performs. `taking_in` and
                // `preset_press` are the key's own two helpers, so this is a
                // second caller and not a second answer.
                //
                // **A folder row is the same press**, and it stopped being
                // refused on 2026-09-08: it is the same two rows of the
                // vocabulary off a directory somebody dropped rather than one
                // the program was told (ADR-0275, ADR-0267), so the two arms
                // are one and [`Taking`] is the difference between them.
                let mut took = Repaint::Never;
                let acted = match (&acted, self.readout.view.scope()) {
                    (
                        Acted::Emitted(Some(Operation::LoadSet { deck, set })),
                        Some(scope @ (Scope::Presets | Scope::Folder)),
                    ) => {
                        let (deck, row) = (*deck, set.clone());
                        let from = match scope {
                            Scope::Folder => Taking::Folder(self.folder.as_deref()),
                            _ => Taking::Presets(self.presets.as_ref()),
                        };
                        match taking_in(&self.store, from, &row) {
                            Ok(taken) => {
                                println!("  take in: {}", taken.said);
                                let [take, load] = taken_in_press(deck, taken);
                                took = App::performed(
                                    gfx,
                                    self.started,
                                    &mut self.readout,
                                    self.recording.recorder(),
                                    &Acted::Emitted(Some(take)),
                                    Repaint::Never,
                                )
                                .repaint;
                                Acted::Emitted(Some(load))
                            }
                            // Which of the two acts failed is the whole of what
                            // this adds — the load's own sentence, one surface over.
                            Err(e) => {
                                println!(
                                    "  take in: `{row}` was not taken into the store: {e}\n  \
                                     take in: so nothing was loaded, and what is on deck {} is \
                                     still running",
                                    deck_letter(deck)
                                );
                                Acted::Nothing
                            }
                        }
                    }
                    _ => acted,
                };
                // **A press on a control earns its frame from what it did**,
                // and not from the claim: `Change::Pointer(Claim::Panel)` is
                // already a frame, but the operation the dot asked for is the
                // thing that moved every region in the Program bay, and it is
                // the outcome that says so.
                // **One frame asked for, however many operations the press
                // emitted** — the load's rule at the pointer, and `Repaint::soonest`
                // is what combines them.
                let repaint = App::performed(
                    gfx,
                    self.started,
                    &mut self.readout,
                    self.recording.recorder(),
                    &acted,
                    Change::Pointer(claim).repaint(),
                )
                .repaint
                .soonest(took);
                App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
            }
            // **The secondary button, and only its press.** A release is not
            // routed at all, which is the whole of what this gesture is: a
            // secondary press puts a row's menu down and takes nothing in
            // hand, so there is nothing for a release to let go of and a
            // `Pointer::Secondary` up would be an event with no arm to run
            // (ADR-0311).
            //
            // **`egui` is not told either way**, which is what this arm
            // changes least: before it, every button but the left one fell
            // through this handler's `_ => {}` and reached nothing, and
            // `egui` owns no widget anywhere on this console, so a secondary
            // press routed to it would reach nothing there either. The claim
            // is asked for the same reason it is asked on a left press —
            // rule 1's drag and rule 2's cards are about the gesture and not
            // about the button — and the answer is used the same way.
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                let ctx = gfx.egui.egui_ctx().clone();
                let (claim, acted) = self.readout.pointer(&ctx, Pointer::Secondary);
                // **The same send branch the left press takes**, because the
                // item is picked by whichever press lands on the card: a menu
                // opened with the secondary button and picked with it again is
                // one gesture, and the second press is the one that names the
                // item.
                if let Acted::Emitted(Some(Operation::TransferSet {
                    transfer: SetTransfer::Send { ref id },
                })) = acted
                {
                    println!("  send: naming a file to write `{id}` to");
                    sending(
                        &gfx.window,
                        &self.store,
                        self.folder.as_deref(),
                        id,
                        self.keeping.send_tx.clone(),
                    );
                }
                let repaint = App::performed(
                    gfx,
                    self.started,
                    &mut self.readout,
                    self.recording.recorder(),
                    &acted,
                    Change::Pointer(claim).repaint(),
                )
                .repaint;
                App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // **The two shapes a wheel arrives in, and only the vertical
                // half of either.** `LineDelta` is a count of detents and is
                // what a mouse sends, so it is multiplied by the console's own
                // `WHEEL_STEP` — three parameter rows, which is what
                // `docs/manual/console.html` says a notch is worth.
                // `PixelDelta` is a trackpad and is already a distance: it is
                // in physical pixels like every other position this handler
                // reads, so it is divided by the scale and passed through.
                //
                // **Negated, because the axes point opposite ways.** `winit`'s
                // positive `y` is a wheel pushed away from the hand, which
                // moves a list *up* — and a scroll position is how far down the
                // content the pane has come.
                let by = match delta {
                    MouseScrollDelta::LineDelta(_, y) => {
                        -y * karakuri_console::room::size::WHEEL_STEP
                    }
                    MouseScrollDelta::PixelDelta(at) => -(at.y / self.scale) as f32,
                };
                let ctx = gfx.egui.egui_ctx().clone();
                let (claim, acted) = self.readout.pointer(&ctx, Pointer::Wheel(by));
                if claim == Claim::Egui {
                    App::to_egui(gfx, &mut self.costs, &event);
                }
                // **The frame is owed for what the wheel moved and not for the
                // claim**, which is the `CursorMoved` arm's own rule one event
                // along: a wheel spun against the top of a pane's list is the
                // panel's and changes nothing, and a frame per notch of that
                // would be a repaint for a gesture with no picture in it.
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Wheeled(claim, acted == Acted::Pointed).repaint(),
                );
            }

            // **The one modifier this loop keeps, and it keeps it for one
            // key.** `shift-Tab` is the tab ring walked backwards (ADR-0259)
            // and `winit`'s `KeyEvent` carries no modifier state, so the
            // answer has to have been listened for — see [`App::shift`].
            //
            // **`egui` is still told**, which is what this arm has to add back:
            // every event this `match` does not name reaches `to_egui` through
            // the wildcard at the bottom, and a modifier taken here and not
            // passed on would leave the toolkit's own copy stale.
            _ => {}
        }
    }
}
