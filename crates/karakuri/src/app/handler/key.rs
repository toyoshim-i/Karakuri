//! Keyboard input and shortcut dispatch.

use winit::event::{ElementState, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};

use karakuri_console::focus;
use karakuri_console::repaint::Change;
use karakuri_operation::{Operation, Undecided};

use super::super::*;
use crate::keymap::{KeyAction, KeyCtx, KEY_BINDINGS};

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
            // **`egui` sees every key, and is never asked for
            // permission.** `App::to_egui` reads `EventResponse::repaint`
            // and nothing else — `consumed` is read nowhere in `crates/` —
            // so the `match` below runs whatever `egui` answers, and
            // `egui`'s modifier state stays current for the frame it does
            // ask for.
            //
            // **Not because nothing has focus.** `egui-winit` 0.36.1
            // hard-codes the flag — *"When pressing the Tab key, egui
            // focuses the first focusable element, hence Tab always
            // consumes"* — so `consumed` is `true` for every `Tab`
            // whatever the focus state is, and honouring it would swallow
            // the first press rather than being harmless. **`Tab` is the
            // key that moves focus between bays here since 2026-09-09**
            // (ADR-0259, ADR-0332), which is where that is the failure
            // that looks like nothing at all; the invariant that record
            // names is `App::to_egui`'s own doc comment, and the shape of
            // that function — nothing past its one destructure holds an
            // `EventResponse` to read `consumed` off — is what enforces
            // it now.
            App::to_egui(gfx, &mut self.costs, &event);
            if key.state != ElementState::Pressed {
                return;
            }
            // **The one flow on this panel that asks for letters takes the
            // keyboard whole while it is asking.** Every key here is a
            // character, a rub-out, the commit or the abandonment, and none
            // of them is the operation that key names the rest of the time
            // — `s` is an `s` in a name and not a solo, and the pointer is
            // not what a name is addressed to. `escape` leaves the program
            // the rest of the time and leaves the name here, which is the
            // same word for the same act one level in.
            //
            // **Nothing typed is checked here**, which is
            // [`checked_name`]'s half of the same split: the pill takes
            // the letters, and the wall is where the file is written.
            // **Inline text input session (Arrangement save or Inspector deck rename)**:
            // Rule 2 claims the keyboard while either flow is active. Handled symmetrically
            // through [`TextInputSession`] (P42) with post-event side-effects decoupled (P46).
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
                // **The four keys of the grammar, dispatched to the bay
                // that has focus** — a digit names the nth thing one level
                // below the address and `0` the bay's head, the arrows take
                // the neighbour or the next value, `space` is the addressed
                // thing's next state and `enter` is the act it is for
                // (ADR-0259).
                //
                // **One arm rather than four**, and that is what makes the
                // re-read below one statement: a digit and an arrow both
                // move the Library's cursor, and the rule that a reading
                // follows it (ADR-0265) is paid once here instead of at
                // each key that could move it.
                //
                // **The console resolves the address and this names the
                // operation** ([ADR-0333](../../../docs/adr/0333-the-console-resolves-the-address-and-the-window-loop-names-the-operation.md)):
                // `karakuri_console::focus::press` knows which control a
                // press landed on and owns the cycle a state goes round
                // (P-0090); what it cannot know is the value the deck is
                // holding, the tenth a level steps by, or what a load costs
                // — so those stay here, where they were.
                //
                // **Every write still goes through the method that already
                // refused it.** A digit that names a strip is
                // `View::select` and one that names a row is
                // `View::point_at`, so a deck the mixer draws no strip for
                // and a row past the listing are turned down exactly where
                // they were before. The grammar adds a route and no
                // exception.
                //
                // **Twelve letters went with it**, and none of them was
                // unbound before the grammar reached the same row:
                // `0`–`3` are the Mixer's `1`–`4`, `[ ] \\` and `; '` are
                // the arrows and `space` on the addressed trim and fader,
                // `m` is `space` on the blend chip, `e` is `space` on the
                // Library's head and `l` is `enter` on one of its rows.
                named if grammar(&named).is_some() => {
                    let press = grammar(&named).expect("the arm this is in");
                    // **What the cursor was on before the press**, so the
                    // re-read below is a *move* and not a press — the
                    // console's own answer would say which of six things
                    // happened and this asks the one question the rule is
                    // about.
                    let was = self.readout.view.cursor_row();
                    // **The deck read here and handed in**, which is the
                    // three mix keys' own rule arriving at the grammar:
                    // `view::Strip` is that same reading copied once a
                    // frame, and a scheduled fade landing between the frame
                    // and the press would cycle from a state the deck has
                    // already left behind. The closure is asked only for
                    // the strip the address is on, and only where a press
                    // needs a state to cycle from.
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
                // **Everything else this loop binds is one lookup into
                // `KEY_BINDINGS`** — the ten literal keys it used to spell
                // as ten arms, now a table checked against
                // `docs/manual/operations.html` by `key_column` directly
                // (see [`KEY_BINDINGS`] for why a table and not arms, and
                // for the doc comment each arm here used to carry).
                other => {
                    let focused_bay = self
                        .readout
                        .view
                        .focused(&self.readout.panel)
                        .map(|b| b.name);
                    match KEY_BINDINGS.iter().find(|binding| {
                        if !binding.key.matches(&other) {
                            return false;
                        }
                        match binding.bay {
                            None => true,
                            Some(focus::ANY) => true,
                            Some(bay) => focused_bay == Some(bay),
                        }
                    }) {
                        Some(binding) => {
                            // **Disjoint fields, not `self`.** `gfx` is
                            // already a live `&mut` borrow out of `self.gfx`
                            // (see the top of `window_event`), so a bound
                            // action takes exactly the other fields it
                            // needs rather than all of `self` — the same
                            // reason `App::performed` and `App::wants` never
                            // took `&mut self` either.
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
        // **Emitted whether or not the mark moved**, which is
        // the deck keys' rule: what a press asked for is what
        // is emitted, and `unwritten` is what says the press
        // wrote no record and that it is settled.
        //
        // **And nothing performs it in `performed`**, where
        // `SelectDeck` has `pointed` — because this payload
        // cannot say which scope was chosen and a performer
        // reading `Undecided` would have to guess. The step
        // above *is* the performance, and it is the surface's
        // own pointer either way.
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
        // **The Sets, which is empty under `history`**: a row
        // of that scope is a version and this key loads a Set,
        // so the arm below names it rather than this line
        // handing a word no store holds to a load
        // (`view::View::sets`).
        let row = readout.view.sets().get(at).cloned();
        // Whose history, for `why_nothing`'s `history` arm —
        // which this key cannot reach, because the arm below
        // answers that scope first, and which is passed anyway
        // because a sentence chosen by a caller is a sentence
        // that can be chosen wrongly.
        let running = aimed_set(gfx, &readout.view).is_some();
        // **What the take-in half of this press asked for**,
        // where a press that took nothing in leaves it
        // `Repaint::Never` — see the preset arm below for why
        // one press emits two operations and why they cannot
        // be one `Acted`.
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
                        // **The take-in is named as well as
                        // performed, and that is `e`'s rule
                        // one key along**: the scope step
                        // emits `SelectScope` *"so that the
                        // press is recorded as `Silent` rather
                        // than as nothing at all"*, and this
                        // press has just performed a whole row
                        // of the vocabulary —
                        // `docs/manual/operations.html`'s
                        // *Send a Set to somebody, and take
                        // one in*, *"opening a preset is this
                        // row"*. Emitting only the load would
                        // be a press that does two of the
                        // page's rows and names one.
                        //
                        // **Two emissions rather than one**,
                        // because they are two rows: taking in
                        // is what gives the Set the id, and
                        // the load names that id. `Acted`
                        // carries one operation — a fader
                        // drag, a chip, a key each emit
                        // exactly one — so the pair is two
                        // trips through `App::performed`
                        // rather than a shape invented here
                        // for the one press that has two.
                        //
                        // **In the order they happened.**
                        // `written` answers
                        // `Silent(NoRecord)` for the transfer,
                        // so nothing in `performed` performs
                        // it and the emission is the naming;
                        // the load after it is what re-points
                        // the slot.
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
                    // **Which of the two acts failed is the
                    // whole of what this sentence adds.**
                    // Nothing was taken in, so nothing was
                    // loaded — where a load that fails says so
                    // in `played`'s own words, with the deck
                    // it did not reach. The refusal itself is
                    // `setfile::unbundle`'s, including the one
                    // for an id this store already holds.
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
            // **A row of `history` is a version and not a
            // Set**, so this key has nothing to load and says
            // so rather than falling through to the sentence
            // below, which would report a listing as empty
            // while it is drawing rows. What lands a version is
            // a press on the row itself —
            // `Operation::RestoreProcedure`, on the deck the
            // load pulldown names rather than on the selection.
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
            // Not a refusal of the load: there is no Set under
            // the cursor because this scope lists nothing. The
            // bay says so by drawing no rows; this says so in
            // words, and it says **which** nothing it is —
            // `favourites`, a folder nobody has pointed
            // anywhere and a folder holding nothing are three
            // different reasons, and a key that did nothing and
            // a key that is not bound are the same experience.
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
        // **One frame asked for, however many operations the
        // press emitted.** `App::wants` counts a frame against
        // the run's costs and asks the window for a redraw, so
        // calling it twice for one press would ask for two
        // frames where one is drawn. `Repaint::soonest` is
        // what combines them, and its own rule is why it is
        // safe: it can only bring a frame forward.
        let repaint = App::performed(gfx, started, readout, recorder, &acted, Repaint::Never)
            .repaint
            .soonest(took);
        App::wants(gfx, egui_due, costs, repaint);
    }
}
