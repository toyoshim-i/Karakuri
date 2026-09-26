use super::*;
use crate::{ir_layer, node_addr, showing};

impl Readout {
    /// Dispatches audio input pill or card interactions, enumerating devices on card open (P-0091).
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
            // Forward audio input attachment operation to app handler (ADR-0156).
            AudioAsk::Operation(operation) => {
                audio.shut();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// Dispatches arrangement pill actions, emitting operations or resetting console state (ADR-0208).
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
            // Prompt for new arrangement name when no arrangement is active.
            Ask::Name => {
                println!(
                    "arrangement: type a name and press return — letters, digits, `-` and \
                     `_`, and escape leaves it unsaved"
                );
                self.view.arrangement.asks_a_name();
                Acted::Nothing
            }
            // Reset arrangement to default layout via Readout::op.
            Ask::Panel(op) => {
                self.view.arrangement.shut();
                Acted::Operated(self.op(op))
            }
            // Emit arrangement operation to the store handler.
            Ask::Operation(operation) => {
                self.view.arrangement.shut();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// Dispatches input wiring card interactions, toggling the dropdown card or emitting wire operations (ADR-0329).
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
            // Dismisses card, rewires input, and emits operation to engine bridge.
            Wiring::Pick(operation) => {
                self.view.shut_wiring();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// Dispatches pane head dropdown interactions, emitting pane repointing operations (ADR-0338).
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
            // Dismiss targeting card and emit pane re-pointing operation.
            view::Pointing::Pick(operation) => Acted::Emitted(Some(operation)),
        }
    }

    /// Dispatches a target deck selection or load command from the Library bay (ADR-0305).
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
            // Update target deck selection without emitting an operation.
            Aim::Deck(deck) => {
                self.view.aim_at(deck);
                Acted::Nothing
            }
            // Emit LoadSet operation onto targeted deck (ADR-0228).
            Aim::Load(operation) => Acted::Emitted(Some(operation)),
            // Report refusal when attempting to load without a selected Set (P-0083).
            Aim::NoSet => {
                println!("load: nothing under the cursor — this library is listing no Sets");
                Acted::Nothing
            }
        }
    }

    /// Dispatches master chain `+ add` card interactions to append effects.
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

    /// Dispatches sequencer `+ lane` card interactions to add a driven lane.
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

    /// Dispatches library row context menu selections, dismissing the menu and emitting load/save operations.
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
            // Emit LoadSet operation from context menu (ADR-0228).
            Picked::Load(operation) => {
                self.view.shut_menu();
                Acted::Emitted(Some(operation))
            }
            // Emit SendSet operation to export set bundle (P-0091, ADR-0156).
            Picked::Send(operation) => {
                self.view.shut_menu();
                Acted::Emitted(Some(operation))
            }
        }
    }

    /// Emits a `SaveArrangement` operation when inline arrangement naming is committed (P-0090).
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

    /// Dispatches program bay solo pill press, toggling program view solo state (ADR-0295, ADR-0300).
    pub(crate) fn folded(&mut self, op: Op) -> Outcome {
        match op {
            Op::Fold(id) => {
                println!("fold: {} folds away — `z` brings it back", self.label(id));
            }
            Op::Unfold(id) => {
                println!("unfold: {} restored", self.label(id));
            }
            other => unreachable!("a fold control asked for {other:?}"),
        }
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

    /// Toggles operation class permissions directly in the environment map without creating operations (ADR-0236).
    pub(crate) fn opened(&mut self, pill: &McpPill) -> Acted {
        // Transition MCP class permission to next state.
        let next: Open = pill.next(self.view.opening);
        self.opening.set(next);
        self.view.opening = next;
        let open = next.holds(pill.class);
        // Log updated class state using standard status messages (P-0090).
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
            if let Some(root) = &self.store_root {
                if let Ok(store) = karakuri_store::Store::open(root) {
                    let names: Vec<&'static str> =
                        self.view.slot_policies.iter().map(|p| p.name()).collect();
                    let _ = store.write_policies(&names);
                }
            }
            println!(
                "slot {}: MCP policy set to {:?} (`{}`)",
                deck + 1,
                next,
                next.pill_word()
            );
        }
        Acted::Opened
    }

    /// Selects library scope filter, emitting `SelectScope` and requesting listing reload (P-0091).
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
        // Walks history scoped to the currently aimed Set (ADR-0308).
        Acted::Emitted(Some(chosen.asked(self.view.aimed.as_deref())))
    }

    /// Updates library filter state in `View` and emits `ListSets` or `FilterLibrary` (P-0090, P-0091).
    pub(crate) fn narrowed(&mut self, operation: Operation) -> Acted {
        let holds = match &operation {
            Operation::ListSets { holds, .. } => holds.as_deref(),
            // Preserve active holds filter when updating kind filters.
            Operation::FilterLibrary { .. } => self.view.filters().holds,
            _ => {
                unreachable!("the filter row emits `ListSets` and `FilterLibrary` and nothing else")
            }
        };
        let holds = holds.map(str::to_owned);
        let kinds = match &operation {
            Operation::FilterLibrary { kinds } => *kinds,
            // Preserve active kind filters when updating holds filter.
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
                // Redundant filter selection re-queries the store for identical criteria.
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

    /// Dispatches library `params` chip toggle, requesting Set parameter reading on open.
    pub(crate) fn asked_to_read(&mut self, ask: Read) -> Acted {
        match ask {
            Read::Open(operation) => Acted::Emitted(Some(operation)),
            Read::Shut => {
                println!(
                    "read: closed — {}",
                    match self.view.shut_reading() {
                        true => "the list is a list again, and the chip asks for it back",
                        // Reading closed prior to click.
                        false => "there was nothing open",
                    }
                );
                Acted::Nothing
            }
        }
    }

    /// Handles library row press to start a carry gesture and update cursor selection (ADR-0265).
    pub(crate) fn took(&mut self, p: Point, taken: Taken) -> Acted {
        let Taken {
            row,
            set,
            procedure,
        } = taken;
        // Update library row selection before starting carry drag.
        let moved = self.view.point_at(row);
        println!(
            "press ({:.0}, {:.0}): `{set}` is in hand — let it go over a strip to load it there, \
             or anywhere else to load nothing",
            p.x, p.y
        );
        self.panel.carry(p, set, procedure);
        // Return Acted::Pointed if cursor row changed, otherwise Acted::Nothing.
        match moved {
            true => Acted::Pointed,
            false => Acted::Nothing,
        }
    }

    /// Handles history row selection to restore a previous procedure version (ADR-0308).
    pub(crate) fn landed(&mut self, operation: Operation) -> Acted {
        println!(
            "press: {} — the version is written over that node's working copy and its \
             watcher builds it, judged against the budget like any edit",
            match &operation {
                Operation::RestoreProcedure {
                    deck,
                    revision: karakuri_operation::Revision::Picked(version),
                } => format!("`{version}` put back on deck {}", deck_letter(*deck)),
                // Resolves previous procedure version for staging lane candidate (ADR-0326).
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

    /// Handles candidate row selection in the staging lane to keep the compiled build (ADR-0326).
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
        // Toggles frame routing to the selected output.
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
