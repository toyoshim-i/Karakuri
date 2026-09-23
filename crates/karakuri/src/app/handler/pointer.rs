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
        let Some(mut gfx) = self.gfx.take() else {
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
                    App::to_egui(&mut gfx, &mut self.costs, &event);
                }
                // Process operations emitted by pointer drags (e.g. fader moves).
                let repaint = App::performed(
                    &mut gfx,
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
                App::wants(&gfx, &mut self.egui_due, &mut self.costs, repaint);
            }
            // **The pointer left the window**, which is not a move to
            // anywhere: a tip that is up goes, and no dwell is running. Only
            // the hover layer cares — `egui` is told either way, because this
            // event is not one `input::claim` has a rule about.
            WindowEvent::CursorLeft { .. } => {
                App::to_egui(&mut gfx, &mut self.costs, &event);
                let tip = self.hover.left();
                App::wants(
                    &gfx,
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
                self.handle_left_mouse_input(&mut gfx, event_loop, &event, state);
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
                    &mut gfx,
                    self.started,
                    &mut self.readout,
                    self.recording.recorder(),
                    &acted,
                    Change::Pointer(claim).repaint(),
                )
                .repaint;
                App::wants(&gfx, &mut self.egui_due, &mut self.costs, repaint);
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
                    App::to_egui(&mut gfx, &mut self.costs, &event);
                }
                // **The frame is owed for what the wheel moved and not for the
                // claim**, which is the `CursorMoved` arm's own rule one event
                // along: a wheel spun against the top of a pane's list is the
                // panel's and changes nothing, and a frame per notch of that
                // would be a repaint for a gesture with no picture in it.
                App::wants(
                    &gfx,
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
        self.gfx = Some(gfx);
    }

    fn handle_left_mouse_input(
        &mut self,
        gfx: &mut Gfx,
        event_loop: &ActiveEventLoop,
        event: &WindowEvent,
        state: ElementState,
    ) {
        let which = match state {
            ElementState::Pressed => Pointer::Down,
            ElementState::Released => Pointer::Up,
        };
        if state == ElementState::Pressed {
            let cursor = self.readout.panel.cursor();
            if self.hover.hit_global_toggle(cursor) {
                self.hover.toggle_globalize();
                App::wants(gfx, &mut self.egui_due, &mut self.costs, Repaint::Now);
                return;
            }
            if let Some(on) = self.hover.hit_key_badge(cursor) {
                let next = if self.hover.learning_key() == Some(on) {
                    None
                } else {
                    let is_glob = karakuri_console::hover::descriptor_at(on)
                        .and_then(|d| d.operation_title)
                        .and_then(|t| self.keymap.find_binding_by_title(t))
                        .map(|b| b.globalize)
                        .unwrap_or(false);
                    self.hover.set_key_globalize(is_glob);
                    Some(on)
                };
                self.hover.set_learning_key(next);
                App::wants(gfx, &mut self.egui_due, &mut self.costs, Repaint::Now);
                return;
            }
            if self.hover.hit_tip_box(cursor) {
                return;
            }
        }
        let prev_focus = self
            .readout
            .view
            .focused(&self.readout.panel)
            .map(|r| r.name);
        let ctx = gfx.egui.egui_ctx().clone();
        let (claim, acted) = self.readout.pointer(&ctx, which);
        let focus_moved = self
            .readout
            .view
            .focused(&self.readout.panel)
            .map(|r| r.name)
            != prev_focus;
        if claim == Claim::Egui {
            App::to_egui(gfx, &mut self.costs, event);
        }
        if let Acted::Emitted(Some(Operation::RouteFrame { output, on })) = acted {
            if let Some(line) = routed(gfx, event_loop, output, on) {
                println!("{line}");
            }
        }
        if let Acted::Emitted(Some(ref operation @ Operation::SetFavourite { .. })) = acted {
            if let Some(line) = favourite(&self.store, Asked::Operator, operation) {
                println!("{line}");
            }
        }
        if matches!(
            acted,
            Acted::Emitted(Some(
                Operation::SelectScope { .. }
                    | Operation::WalkHistory { .. }
                    | Operation::ListSets { .. }
                    | Operation::SetFavourite { .. }
            ))
        ) {
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
        if let Acted::Emitted(Some(Operation::KeepProcedure { deck, node, ref id })) = acted {
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
        if let Acted::Emitted(Some(Operation::RecordSession { ref recording })) = acted {
            self.recording
                .asked(&self.keeping, &gfx.engine, &self.store, recording);
        }
        if let Some(line) = reread_if_open(
            matches!(acted, Acted::Pointed),
            &mut self.readout.view,
            &self.store,
        ) {
            println!("{line}");
        }
        let (acted, took) = self.handle_pointer_drop_load(gfx, acted);
        let mut repaint = App::performed(
            gfx,
            self.started,
            &mut self.readout,
            self.recording.recorder(),
            &acted,
            Change::Pointer(claim).repaint(),
        )
        .repaint
        .soonest(took);
        if focus_moved {
            repaint = repaint.soonest(Change::Pointed(true).repaint());
        }
        App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
    }

    fn handle_pointer_drop_load(&mut self, gfx: &mut Gfx, acted: Acted) -> (Acted, Repaint) {
        match (&acted, self.readout.view.scope()) {
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
                        let took = App::performed(
                            gfx,
                            self.started,
                            &mut self.readout,
                            self.recording.recorder(),
                            &Acted::Emitted(Some(take)),
                            Repaint::Never,
                        )
                        .repaint;
                        (Acted::Emitted(Some(load)), took)
                    }
                    Err(e) => {
                        println!(
                            "  take in: `{row}` was not taken into the store: {e}\n  \
                             take in: so nothing was loaded, and what is on deck {} is \
                             still running",
                            deck_letter(deck)
                        );
                        (Acted::Nothing, Repaint::Never)
                    }
                }
            }
            _ => (acted, Repaint::Never),
        }
    }
}
