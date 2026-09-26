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
                // Notify hover layer of pointer movement; redraw is requested immediately only if an invalid tip must hide.
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
            // Clear hover tip and forward leave event to egui.
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
            // Secondary button press opens context menus; releases and egui forwarding are skipped as they carry no state (ADR-0311).
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                let ctx = gfx.egui.egui_ctx().clone();
                let (claim, acted) = self.readout.pointer(&ctx, Pointer::Secondary);
                // Card clicks via secondary press trigger selection, matching left-click behavior.
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
                // Convert vertical scroll delta to logical pixels (`LineDelta` scaled by `WHEEL_STEP`, `PixelDelta` scaled by DPI).
                // Negate delta so positive wheel deflection scrolls down content.
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
                // Request redraw only if the wheel delta actually shifted scroll offset.
                App::wants(
                    &gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Wheeled(claim, acted == Acted::Pointed).repaint(),
                );
            }

            // Track Shift modifier state for reverse tab cycling (ADR-0259) while forwarding to egui ([`App::shift`]).
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
