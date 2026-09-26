//! Keyboard input and shortcut dispatch.

use winit::event::{ElementState, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};

use karakuri_console::focus;
use karakuri_console::repaint::Change;
use karakuri_operation::{Operation, Undecided};

use super::super::*;
use crate::keymap::{KeyAction, KeyCtx};

impl App {
    pub(crate) fn handle_keyboard_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        event: WindowEvent,
    ) {
        let Some(gfx) = self.gfx.as_mut() else {
            return;
        };
        if let WindowEvent::KeyboardInput { event: ref key, .. } = event {
            // Forward every key to egui without consulting `consumed`, ensuring modifiers
            // stay current and `Tab` focus switching between bays is not swallowed (ADR-0259, ADR-0332).
            App::to_egui(gfx, &mut self.costs, &event);
            if key.state != ElementState::Pressed {
                return;
            }

            // Interactive Tooltip Key Learn Mode: capture next key to bind to the active control.
            if let Some(on) = self.hover.learning_key() {
                if key.logical_key == Key::Named(NamedKey::Escape) {
                    self.hover.set_learning_key(None);
                    App::wants(gfx, &mut self.egui_due, &mut self.costs, Repaint::Now);
                    return;
                }
                let key_str = match &key.logical_key {
                    Key::Character(s) => Some(s.as_str()),
                    Key::Named(NamedKey::Space) => Some("space"),
                    _ => None,
                };
                if let Some(key_str) = key_str {
                    let desc = karakuri_console::hover::descriptor_at(on);
                    let ctrl_id = karakuri_console::hover::control_id_at(on);
                    let action_opt = ctrl_id.and_then(|id| {
                        crate::keymap::ActionId::for_control(
                            id,
                            desc.and_then(|d| d.operation_title),
                        )
                    });
                    if let Some(action_id) = action_opt {
                        let bay = match desc.map(|d| d.eyebrow) {
                            Some("TRANSPORT") => Some("transport"),
                            Some("MIXER") => Some("mixer"),
                            Some("INSPECTOR") => Some("inspector"),
                            Some("LIBRARY") => Some("library"),
                            Some("STAGING") => Some("staging"),
                            _ => None,
                        };
                        let bay = if action_id == crate::keymap::ActionId::FoldEnclosing
                            || action_id == crate::keymap::ActionId::UnfoldAll
                        {
                            Some("any")
                        } else {
                            bay
                        };
                        let globalize = self.hover.key_globalize();
                        match self.keymap.bind_action(
                            &self.store,
                            action_id,
                            bay,
                            key_str,
                            globalize,
                        ) {
                            Ok(notes) => {
                                for note in notes {
                                    eprintln!("keymap: {note}");
                                }
                            }
                            Err(why) => eprintln!("keymap: failed to bind: {why}"),
                        }
                    }
                }
                self.hover.set_learning_key(None);
                App::wants(gfx, &mut self.egui_due, &mut self.costs, Repaint::Now);
                return;
            }
            // Inline text input session (Arrangement save or Inspector deck rename) claims
            // all keystrokes symmetrically through [`TextInputSession`] (P42, P46) until committed or cancelled.
            if let Some(session) = self.readout.view.active_text_input() {
                Self::handle_active_text_input(
                    gfx,
                    self.started,
                    &mut self.readout,
                    self.recording.recorder(),
                    &mut self.egui_due,
                    &mut self.costs,
                    &mut self.keeping,
                    &self.store,
                    key,
                    session,
                );
                return;
            }
            let op = match key.logical_key.as_ref() {
                // Dispatch focus grammar keys (digits, arrows, space, enter) to the focused bay.
                // The console resolves the target address while this loop names the operation (ADR-0259, ADR-0265, ADR-0333).
                named if grammar(&named, self.alt, self.ctrl).is_some() => {
                    let press = grammar(&named, self.alt, self.ctrl).expect("the arm this is in");
                    // Record cursor position before the move so the console can distinguish a movement from an in-place action.
                    let was = self.readout.view.cursor_row();
                    // Read active deck state on demand to prevent cycling from stale strip frame snapshots.
                    let asked =
                        focus::press(&mut self.readout.view, &self.readout.panel, press, |deck| {
                            holding(&gfx.engine.deck, deck)
                        });
                    let moved = self.readout.view.cursor_row() != was;
                    // ADR-0265: the reading follows the cursor, on
                    // whichever surface moved it — see [`reread_if_open`],
                    // the pointer release's own call one arm up.
                    if let Some(line) = reread_if_open(moved, &mut self.readout.view, &self.store) {
                        println!("{line}");
                    }
                    match asked {
                        focus::Asked::Scope => {
                            Self::handle_grammar_scope(
                                gfx,
                                self.started,
                                &mut self.readout,
                                self.recording.recorder(),
                                &mut self.egui_due,
                                &mut self.costs,
                                &self.store,
                                self.presets.as_ref(),
                                self.folder.as_deref(),
                            );
                            return;
                        }
                        focus::Asked::Load => {
                            Self::handle_grammar_load(
                                gfx,
                                self.started,
                                &mut self.readout,
                                self.recording.recorder(),
                                &mut self.egui_due,
                                &mut self.costs,
                                &self.store,
                                self.presets.as_ref(),
                                self.folder.as_deref(),
                            );
                            return;
                        }
                        asked => {
                            let repaint = answered(
                                gfx,
                                self.started,
                                &mut self.readout,
                                self.recording.recorder(),
                                &asked,
                            );
                            App::wants(gfx, &mut self.egui_due, &mut self.costs, repaint);
                            return;
                        }
                    }
                }
                // Remaining key bindings are resolved via [`KEY_BINDINGS`] table lookup.
                other => {
                    let focused_bay = self
                        .readout
                        .view
                        .focused(&self.readout.panel)
                        .map(|b| b.name);
                    match self.keymap.find_binding(&other, focused_bay) {
                        Some(binding) => {
                            // Pass disjoint fields to avoid overlapping with the active `&mut self.gfx` borrow.
                            let mut ctx = KeyCtx {
                                readout: &mut self.readout,
                                egui_due: &mut self.egui_due,
                                costs: &mut self.costs,
                                recording: &mut self.recording,
                                keeping: &mut self.keeping,
                                store: &self.store,
                                started: self.started,
                                shift: self.shift,
                            };
                            match binding.action {
                                KeyAction::Handled(act) => {
                                    act(&mut ctx, gfx);
                                    return;
                                }
                                KeyAction::Focus(act) => {
                                    let moved = act(&mut ctx);
                                    App::wants(
                                        gfx,
                                        &mut self.egui_due,
                                        &mut self.costs,
                                        Change::Pointed(moved).repaint(),
                                    );
                                    return;
                                }
                                KeyAction::Panel(act) => match act(&mut ctx) {
                                    Some(op) => op,
                                    None => return,
                                },
                            }
                        }
                        // Not one of the grammar's four and not in the table
                        // either — an unbound key, answered with nothing.
                        None => return,
                    }
                }
            };
            let outcome = self.readout.op(op);
            App::wants(
                gfx,
                &mut self.egui_due,
                &mut self.costs,
                Change::Operated(&outcome).repaint(),
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_active_text_input(
        gfx: &mut Gfx,
        started: Instant,
        readout: &mut Readout,
        recorder: Option<&mut karakuri_environment::session::Recorder>,
        egui_due: &mut Option<Instant>,
        costs: &mut Costs,
        keeping: &mut Keeping,
        store: &std::path::Path,
        key: &winit::event::KeyEvent,
        session: TextInputKind,
    ) {
        let (acted, moved) = match key.logical_key.as_ref() {
            Key::Named(NamedKey::Escape) => {
                match session {
                    TextInputKind::Arrangement => {
                        println!("arrangement: nothing was saved");
                    }
                    TextInputKind::DeckName(_) => {
                        println!("inspector: nothing was kept");
                    }
                }
                readout.view.cancel_active_text_input();
                (Acted::Nothing, true)
            }
            Key::Named(NamedKey::Enter) => (readout.commit_active_text_input(), true),
            Key::Named(NamedKey::Backspace) => (Acted::Nothing, readout.view.rub_out_active()),
            Key::Character(text) => (Acted::Nothing, readout.view.type_into_active(text)),
            Key::Named(NamedKey::Space) => (Acted::Nothing, readout.view.type_into_active(" ")),
            _ => (Acted::Nothing, false),
        };
        let repaint = App::performed(
            gfx,
            started,
            readout,
            recorder,
            &acted,
            Change::Naming(moved).repaint(),
        )
        .repaint;
        App::wants(gfx, egui_due, costs, repaint);
        handle_post_event_side_effects(keeping, &gfx.engine, store, &mut readout.view, &acted);
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_grammar_scope(
        gfx: &mut Gfx,
        started: Instant,
        readout: &mut Readout,
        recorder: Option<&mut karakuri_environment::session::Recorder>,
        egui_due: &mut Option<Instant>,
        costs: &mut Costs,
        store: &std::path::Path,
        presets: Option<&karakuri_environment::places::Presets>,
        folder: Option<&std::path::Path>,
    ) {
        if readout.view.step_scope() {
            // Whose history, for the press branch's reason:
            // the scope key steps onto the `history` chip
            // as readily as the pointer names it.
            let running = aimed_set(gfx, &readout.view);
            println!(
                "{}",
                listing(
                    &mut readout.view,
                    store,
                    presets,
                    folder,
                    running.as_deref(),
                )
            );
        }
        // Emit the deck selection operation as unwritten; performance occurs locally on the view surface.
        let acted = Acted::Emitted(Some(Operation::SelectScope { scope: Undecided }));
        let repaint =
            App::performed(gfx, started, readout, recorder, &acted, Repaint::Never).repaint;
        App::wants(gfx, egui_due, costs, repaint);
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_grammar_load(
        gfx: &mut Gfx,
        started: Instant,
        readout: &mut Readout,
        mut recorder: Option<&mut karakuri_environment::session::Recorder>,
        egui_due: &mut Option<Instant>,
        costs: &mut Costs,
        store: &std::path::Path,
        presets: Option<&karakuri_environment::places::Presets>,
        folder: Option<&std::path::Path>,
    ) {
        let deck = readout.view.selection();
        let at = readout.view.cursor_row();
        // Sets listing is empty under `history`, where rows represent versions rather than sets (`view::View::sets`).
        let row = readout.view.sets().get(at).cloned();
        // Pass active history scope target for `why_nothing` diagnostics.
        let running = aimed_set(gfx, &readout.view).is_some();
        // Tracks repaint requirement for preset import; defaults to `Repaint::Never` if no import occurred.
        let mut took = Repaint::Never;
        let acted = match (readout.view.scope(), row) {
            // Take in preset or folder item into store and load it (ADR-0229, ADR-0275, ADR-0299).
            (Some(scope @ (Scope::Presets | Scope::Folder)), Some(row)) => {
                let from = match scope {
                    Scope::Folder => Taking::Folder(folder),
                    _ => Taking::Presets(presets),
                };
                match taking_in(store, from, &row) {
                    Ok(taken) => {
                        println!("  take in: {}", taken.said);
                        // Emits two sequential operations for preset open: import (transfer) followed by load,
                        // recording both vocabulary actions in `App::performed`.
                        let [take, load] = taken_in_press(deck, taken);
                        took = App::performed(
                            gfx,
                            started,
                            readout,
                            recorder.as_deref_mut(),
                            &Acted::Emitted(Some(take)),
                            Repaint::Never,
                        )
                        .repaint;
                        Acted::Emitted(Some(load))
                    }
                    // Report import failure from `setfile::unbundle` before attempting any load.
                    Err(e) => {
                        println!(
                            "  take in: `{row}` was not taken into the store: \
                             {e}\n  take in: so nothing was loaded, and what is \
                             on deck {} is still running — a Set already here is \
                             listed under `all`, which is where it is loaded \
                             from",
                            deck_letter(deck)
                        );
                        Acted::Nothing
                    }
                }
            }
            // A history row represents a version rather than a loadable Set; versions are restored directly via `Operation::RestoreProcedure`.
            (Some(Scope::History), _) => {
                println!(
                    "  load: `history` lists the versions of a Set rather than \
                     Sets, so there is nothing here for enter to load — press a \
                     row to put that version back on its node, or mark `all` \
                     and load a Set"
                );
                Acted::Nothing
            }
            // A row of `all` or of `my sets`, which is a Set
            // this store already holds and is the route
            // ADR-0228 built.
            (_, Some(set)) => Acted::Emitted(Some(Operation::LoadSet { deck, set })),
            // Explain why no Set was loaded when the active scope contains no items.
            (scope, None) => {
                println!(
                    "  load: `{}` lists nothing, so there is no Set under the \
                     cursor — {}",
                    match scope {
                        Some(scope) => scope.name(),
                        None => "the library",
                    },
                    match scope {
                        Some(scope) => why_nothing(scope, folder.is_some(), running),
                        None => "this console was handed no scopes at all",
                    }
                );
                Acted::Nothing
            }
        };
        // Request a single redraw via `Repaint::soonest` even if multiple operations were emitted.
        let repaint = App::performed(gfx, started, readout, recorder, &acted, Repaint::Never)
            .repaint
            .soonest(took);
        App::wants(gfx, egui_due, costs, repaint);
    }
}
