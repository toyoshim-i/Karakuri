use super::super::*;

impl Live {
    pub(crate) fn focus_slot(&mut self, slot: usize) {
        if !slot_in_range(slot, self.deck.slot_count()) {
            eprintln!("{}", no_such_slot(slot, self.deck.slot_count()));
            return;
        }
        self.focus = slot;
        let addr = EngineSlot(slot as u8);
        eprintln!(
            "focus slot {slot} — {}, gain {:.2}, t {:.2}s",
            residency_name(self.deck.residency(addr), self.deck.is_parked(addr)),
            self.deck.gain(addr),
            self.deck.slot(addr).set().time()
        );
    }

    /// On air and off again. `Allocated` frees nothing and resets nothing, so the
    /// `t` printed on the way out is the `t` printed on the way back in — which is
    /// the whole property, and printing both ends is the only way to see it without
    /// a debugger.
    pub(crate) fn toggle_focused(&mut self) {
        self.toggle_on_air(self.focus);
    }

    /// On air and off again, for a named slot.
    ///
    /// The toggle is the key's affordance and not an operation, which is why the
    /// slot is a parameter and the focus is filled in by the caller: what reaches
    /// the deck is `SetResidency` naming one of three
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`). A control
    /// surface says which state it wants on the line and does not come through here
    /// at all — it used to, and the state it landed in was this function's to
    /// decide.
    pub(crate) fn toggle_on_air(&mut self, slot: usize) {
        let addr = EngineSlot(slot as u8);
        let t = self.deck.slot(addr).set().time();
        match self.deck.residency(addr) {
            Residency::Live => {
                self.operate(&Operation::SetResidency {
                    deck: slot as u8,
                    residency: mix::residency(Residency::Allocated),
                });
                eprintln!("slot {slot} off air — allocated, holding t {t:.2}s");
            }
            // Priming is warming out of sight and is still off air, so space
            // does the same thing to it: puts it on, at whatever `t` it has
            // warmed to.
            Residency::Allocated | Residency::Priming => {
                self.operate(&Operation::SetResidency {
                    deck: slot as u8,
                    residency: mix::residency(Residency::Live),
                });
                eprintln!("slot {slot} on air — resuming at t {t:.2}s");
            }
        }
    }

    /// Ask the focused slot to warm out of sight, or withdraw the request.
    ///
    /// A request, not a command — the governor decides whether it is granted, and
    /// the report printed here says which. Whether it takes effect is exactly the
    /// thing the operator cannot otherwise see: a refused request leaves the slot
    /// at Allocated, which is what an untouched slot looks like too.
    ///
    /// Live slots are left alone. Priming is off-air warming, so asking a slot on
    /// air to prime could only mean taking it off air, and that is what space is
    /// for.
    pub(crate) fn toggle_priming(&mut self, slot: usize) {
        let addr = EngineSlot(slot as u8);
        if self.deck.residency(addr) == Residency::Live {
            eprintln!("slot {slot} is on air — priming is off-air warming; take it off with space");
            return;
        }
        let requested = self.deck.requested_residency(addr);
        let want = if requested == Residency::Priming {
            Residency::Allocated
        } else {
            Residency::Priming
        };
        // Said before the record is applied, because applying it runs a
        // governor pass that prints what it decided — and the decision reads as
        // an answer to a question the operator has not seen asked otherwise.
        eprintln!(
            "slot {slot} {}",
            if want == Residency::Priming {
                "asked to warm off air"
            } else {
                "prime request withdrawn"
            }
        );
        self.operate(&Operation::SetResidency {
            deck: slot as u8,
            residency: mix::residency(want),
        });
    }

    /// Take up what a slot is now playing, and say so in the stream if a session is
    /// being recorded.
    ///
    /// `landed` is the id of the build that went in, and there is no other case: a
    /// swap is the only event that changes what a slot holds (ADR-0316). It used to
    /// take an `Option`, whose `None` was a rollback, and the version that came
    /// back had to have been remembered because the stream would never name it
    /// again.
    ///
    /// The bookkeeping is unconditional and the record is not, and the two used to
    /// be one function that began by returning when there was no recorder. What a
    /// slot is playing was therefore a fact only a recorded run had — and
    /// `Live::save_set` needs exactly that fact in the ordinary `--watch` case,
    /// where nothing is being recorded. Splitting it is the whole of what widening
    /// `watch::Watch::stored` is for on this side of the channel.
    pub(crate) fn took_up(&mut self, slot: usize, landed: u64) {
        // Drained here rather than per frame: the channel only has anything in
        // it when a build has just been requested, and this runs when one has
        // just landed.
        if let Some(rx) = &self.rebuilds {
            while let Ok(built) = rx.try_recv() {
                self.pending_builds.insert(built.id, built);
            }
        }

        // **A build with nothing in `pending_builds` is one whose sources the
        // watcher could not store**, which it said at the time. It is handed to
        // `landed` as `None` rather than returned on, because the swap happened
        // either way: a slot that took a version nobody can name is a slot with
        // no address, not a slot still on its old one. See [`Running::landed`],
        // which is where that used to go wrong.
        let built = self.pending_builds.remove(&landed);
        let Some(nodes) = self.running.landed(slot, built.map(|built| built.nodes)) else {
            // A slot with nothing in the store behind it — one filled from a Set
            // file with nothing watching it, or one that has just taken a build
            // whose sources could not be stored, which was said at the time.
            // There is no hash to name, so there is nothing this could record.
            return;
        };
        self.record_procedure(slot, &nodes);
    }

    /// Emits `Record::Procedure` entries for the active nodes of the given slot.
    pub(crate) fn record_procedure(&mut self, slot: usize, nodes: &Nodes) {
        if self.recorder.is_none() {
            return;
        }
        // One record per node, each at the address the watcher gave it — so a
        // slot with one renderer writes exactly the two lines it always did,
        // and a slot with a chain and two geometries writes a line for each.
        for (layer, index, hash) in nodes {
            let Some(record) = layer_named(layer).map(record_layer) else {
                eprintln!("  a node on layer `{layer}` is not in the record vocabulary");
                continue;
            };
            self.record_only(karakuri_store::record::Record::Procedure {
                slot: DeckSlot(slot as u8),
                at: karakuri_store::record::NodeAddress {
                    layer: record,
                    index: *index,
                },
                proc_hash: *hash,
            });
        }
    }

    /// Push a record without applying it.
    ///
    /// Two records are right here, and they are right for one reason: each
    /// *describes* a change that has already happened rather than asking for one. A
    /// `procedure` says what a slot became when a swap landed, and a `save` says a
    /// file exists — applying either would mean doing the thing a second time.
    /// Everything else goes through `Live::record`, which applies what it wrote.
    pub(crate) fn record_only(&mut self, record: karakuri_store::record::Record) {
        if let Some(recorder) = &mut self.recorder {
            recorder.push(record);
        }
    }

    /// Saves the active state of a slot's Set to disk (ADR-0122).
    ///
    /// Reads parameters, bindings, and active values directly from the live Set,
    /// spawning a background thread to handle disk I/O while outcome notifications
    /// and records are handled on frame completion.
    pub(crate) fn save_set(
        &mut self,
        asked: Asked,
        slot: usize,
        id: Option<String>,
        reply: Option<mcp::Reply>,
    ) {
        // **Checked here rather than only where the request came from.** A key
        // press cannot name a slot this deck does not hold and a tool call can,
        // and below this line `playing_values` reads `deck.slot(slot)`, which
        // panics on one. The surface that asked refuses it too, in its own
        // words, so a model never reaches this — and this is the guard that
        // does not depend on it having.
        if !slot_in_range(slot, self.deck.slot_count()) {
            return refused(reply, no_such_slot(slot, self.deck.slot_count()));
        }
        let startup = self.startup.get(slot).map_or(&[][..], Vec::as_slice);
        let sources = live_sources(self.running.playing(slot), startup);
        if sources.is_empty() {
            return refused(
                reply,
                nothing_to_save(slot, self.loaded_set.as_deref(), startup.is_empty()),
            );
        }
        // **Named and answered above the GPU line**, which is where the whole
        // of [`accepted_save`]'s doc lives: `playing_values` below is the one
        // read here that needs a `Deck`, and the accept has to be on the side
        // of it a test can reach.
        let id = accepted_save(
            slot,
            asked,
            id,
            &sources,
            &self.store_root,
            reply
                .as_ref()
                .map(|r| r as &dyn karakuri_environment::SaveReply),
        );
        let values = playing_values(self.deck.slot(EngineSlot(slot as u8)).set(), &self.edges);
        let save = Save {
            slot,
            asked,
            id,
            root: self.store_root.clone(),
            sources,
            values,
        };
        let tx = self.save_tx.clone();
        // **A thread per save**, and detached: no frame waits for it. The *run*
        // waits, once, at the end and under a bound — see
        // [`Live::awaited_saves`], which is what this count is for.
        self.saves_in_flight += 1;
        std::thread::spawn(move || {
            let (slot, asked, id) = (save.slot, save.asked, save.id.clone());
            let outcome = save.run();
            // **Carried back rather than answered from here.** This thread
            // knows the outcome and could send it, and that would be a second
            // place a save is reported from: the record is written at the frame
            // the outcome lands, and a model told anything else than what that
            // frame decided would be reading a different story from the stream.
            let _ = tx.send(Saved {
                slot,
                asked,
                id,
                outcome,
                reply,
            });
        });
    }

    /// Every save that has landed since the last frame, said and recorded.
    ///
    /// Drained and never waited on: a frame owes the display a picture and owes a
    /// disk nothing.
    ///
    /// Called at the top of the frame, beside [`Live::run_requests`] and above
    /// `frame::compose` and everything downstream of it — not beside the swap-event
    /// drain, which is where it used to be, below the early returns a frame no
    /// longer has. See the comment at the head of [`Live::frame`]: a window that
    /// has faulted still has saves finishing behind it, and a run that told nobody
    /// about them until it quit was withholding the one answer a waiting client
    /// cannot get anywhere else. The swap drain stays below because a swap *is*
    /// about what was drawn; a save is not.
    ///
    /// The record is written here, at the frame the outcome arrived, which is the
    /// pattern `record_procedure` already follows — a record that describes a
    /// change already made. Writing one at the key press would be a stream claiming
    /// a file that the disk then refused, which is the failure this whole codebase
    /// is arranged against.
    pub(crate) fn finished_saves(&mut self) {
        let mut landed: Vec<Saved> = Vec::new();
        while let Ok(saved) = self.saves.try_recv() {
            landed.push(saved);
        }
        for saved in landed {
            self.took_save(saved);
        }
    }

    /// One save's outcome, said, recorded, and answered.
    ///
    /// One sentence for all three. What the terminal is told, what the stream
    /// records and what a waiting client is handed are the same fact, so the words
    /// are formed once here and the client gets the ones the operator got. The
    /// `Ok`/`Err` split is what a tool call's `isError` is built from — see
    /// [`mcp::Reply::settled`].
    pub(crate) fn took_save(&mut self, saved: Saved) {
        let Saved {
            slot,
            asked,
            id,
            outcome,
            reply,
        } = saved;
        self.saves_in_flight = self.saves_in_flight.saturating_sub(1);
        let said = match outcome {
            Ok(()) => {
                // Distinguish between operator library saves and MCP sandbox saves (ADR-0261).
                let said = match asked {
                    Asked::Operator => {
                        format!("slot {slot}: saved as set `{id}` — load it with `--load-set {id}`")
                    }
                    Asked::Model => format!(
                        "slot {slot}: saved as set `{id}` in the sandbox — \
                         `<store>/{}/{id}{}`. A save asked for over MCP is kept there \
                         rather than in the operator's library, so `--load-set {id}` does \
                         not reach it; the operator's own `k` writes the library",
                        karakuri_store::store::Store::SANDBOX,
                        karakuri_store::store::Store::SET_FILE_SUFFIX,
                    ),
                };
                eprintln!("{said}");
                self.record_only(karakuri_store::record::Record::Save {
                    slot: DeckSlot(slot as u8),
                    id,
                });
                Ok(said)
            }
            // **Printed, and nothing written.** See `Saved::outcome`.
            Err(e) => {
                let said = format!("slot {slot}: set `{id}` was not saved: {e}");
                eprintln!("{said}");
                Err(said)
            }
        };
        if let Some(reply) = reply {
            reply.settled(said);
        }
    }

    /// Every save still being written, waited for — up to [`SAVE_WAIT`].
    ///
    /// A frame owes the disk nothing, which is why [`finished_saves`] drains and
    /// never blocks. The end of the run is the one moment where that is the wrong
    /// trade: a save pressed in the last second reached the disk under an id
    /// nothing in the stream ever named, so the claim that a `save` record exists
    /// for every live save that reached the disk
    /// (`docs/adr/0120-a-record-may-reach-outside-the-stream.md`) was false in
    /// exactly the window an operator is most likely to be in — press `k`, see it
    /// took, quit.
    ///
    /// Bounded, because a disk can hang and quitting must not depend on one. The
    /// alternative was joining the threads, which is unbounded by construction: a
    /// store on a network mount that stops answering would take the window with it.
    /// Past the bound the run says how many saves it left behind and exits, which
    /// is the same trade the recorder makes when it counts the batches it lost
    /// rather than waiting for them.
    ///
    /// The wait is only ever paid by a run that pressed `k` and quit within a few
    /// frames; the count is zero for every other run and this returns without
    /// blocking.
    pub(crate) fn awaited_saves(&mut self) {
        self.finished_saves();
        if self.saves_in_flight == 0 {
            return;
        }
        eprintln!(
            "waiting up to {:.0}s for {} save{} still being written",
            SAVE_WAIT.as_secs_f32(),
            self.saves_in_flight,
            if self.saves_in_flight == 1 { "" } else { "s" }
        );
        let deadline = Instant::now() + SAVE_WAIT;
        for saved in drained_saves(&self.saves, self.saves_in_flight, deadline) {
            self.took_save(saved);
        }
        if self.saves_in_flight > 0 {
            eprintln!(
                "  {} save{} still unfinished after {:.0}s — each is written or it is not, \
                 and no record claims either way",
                self.saves_in_flight,
                if self.saves_in_flight == 1 { "" } else { "s" },
                SAVE_WAIT.as_secs_f32(),
            );
        }
    }
}
