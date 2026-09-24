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

    /// Toggles on-air residency for the specified slot.
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

    /// Requests or cancels off-air priming for the focused slot.
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

    /// Updates active slot procedure metadata and records landed build in the session stream.
    pub(crate) fn took_up(&mut self, slot: usize, landed: u64) {
        // Drained here rather than per frame: the channel only has anything in
        // it when a build has just been requested, and this runs when one has
        // just landed.
        if let Some(rx) = &self.rebuilds {
            while let Ok(built) = rx.try_recv() {
                self.pending_builds.insert(built.id, built);
            }
        }

        // Track landed swap even if sources could not be stored in the watcher.
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

    /// Pushes a record to the recorder without re-applying its changes.
    ///
    /// Used for records that describe already-committed state (e.g., completed swaps or saves).
    pub(crate) fn record_only(&mut self, record: karakuri_store::record::Record) {
        if let Some(recorder) = &mut self.recorder {
            recorder.push(record);
        }
    }

    /// Spawns a background task to save the active state of a slot's Set to disk (ADR-0122).
    pub(crate) fn save_set(
        &mut self,
        asked: Asked,
        slot: usize,
        id: Option<String>,
        reply: Option<mcp::Reply>,
    ) {
        // Validate slot index prior to reading live values.
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
        self.saves_in_flight += 1;
        std::thread::spawn(move || {
            let (slot, asked, id) = (save.slot, save.asked, save.id.clone());
            let outcome = save.run();
            // Outcome is sent back to the main thread so recording and replies synchronize with frame execution.
            let _ = tx.send(Saved {
                slot,
                asked,
                id,
                outcome,
                reply,
            });
        });
    }

    /// Processes all saves that have completed since the last frame.
    ///
    /// Non-blocking drain of completed save operations. Records successful saves into
    /// the session log and dispatches replies to awaiting callers.
    pub(crate) fn finished_saves(&mut self) {
        let mut landed: Vec<Saved> = Vec::new();
        while let Ok(saved) = self.saves.try_recv() {
            landed.push(saved);
        }
        for saved in landed {
            self.took_save(saved);
        }
    }

    /// Handles a single completed save operation, printing status, recording it, and replying.
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

    /// Waits up to [`SAVE_WAIT`] for any in-flight saves to finish before shutdown.
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
