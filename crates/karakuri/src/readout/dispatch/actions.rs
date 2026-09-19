use super::*;
use crate::{ir_layer, node_addr, showing};

impl Readout {
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
