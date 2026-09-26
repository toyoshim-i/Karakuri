use std::time::Instant;

use crate::{
    built_nodes, copied, deck_letter, ir_layer, node_addr, playing_values, refused, slot_in_range,
    Aiming, Engine, Kept, Playing, Save, Saved, Sent,
};
use karakuri_engine::DeckSlot as EngineSlot;
use karakuri_environment::{session, setfile, watch, Asked};
use karakuri_mcp as mcp;

use super::recording::{addressed, Starting};
use super::watch::rewired;
use super::HEAD_SLOT;

// ---------------------------------------------------------------------------
// Keeping what a deck is playing, and rewiring it
// ---------------------------------------------------------------------------

/// Runtime state for persisting deck sets, keeping procedures, and rewiring.
///
/// Groups channels and playing state passed into event handlers while graphics
/// context is borrowed. Constructed directly by `App::new` and test fixtures.
pub(crate) struct Keeping {
    /// MCP server reporter channel when `--mcp` is active.
    ///
    /// Communicates swap events and client requests to the MCP server.
    pub(crate) mcp: Option<mcp::Reporter>,
    /// Thread-safe slot MCP modification policies.
    pub(crate) slot_policies: karakuri_environment::SlotPolicies,
    /// What each deck is playing, seeded before the first frame and moved by every
    /// build that lands — see [`Playing`].
    pub(crate) playing: Playing,
    /// Where the watchers report what they built and stored — the other end of
    /// [`watch::Watch::storing_to`], drained where a build lands.
    pub(crate) built: std::sync::mpsc::Receiver<watch::Built>,
    /// Builds reported by watchers awaiting swap completion at a frame boundary.
    ///
    /// At most one pending build per slot is retained until swapped or dropped.
    pub(crate) pending: Vec<watch::Built>,
    /// Where a save that has reached the disk comes back, and the sending half each
    /// save thread is given a clone of.
    pub(crate) saves: std::sync::mpsc::Receiver<Saved>,
    pub(crate) save_tx: std::sync::mpsc::Sender<Saved>,
    /// Channel for file-dialog export completions (`Sent`).
    ///
    /// Unlike store saves, dialog exports are not awaited at application exit.
    pub(crate) sends: std::sync::mpsc::Receiver<Sent>,
    pub(crate) send_tx: std::sync::mpsc::Sender<Sent>,
    /// Channel for procedure `.kir` keep completions (`Kept`).
    ///
    /// Separated from set saves to maintain dedicated queues and distinct error semantics.
    pub(crate) keeps: std::sync::mpsc::Receiver<Kept>,
    pub(crate) keep_tx: std::sync::mpsc::Sender<Kept>,
    /// How many saves are being written right now. The run waits for these once, at
    /// the end and under a bound — see [`Keeping::awaited_saves`].
    pub(crate) in_flight: usize,
}

impl Keeping {
    /// Processes pending MCP save and wire requests from the background channel.
    pub(crate) fn requests(&mut self, engine: &mut Engine, root: &std::path::Path) {
        let Some(mcp) = &self.mcp else {
            return;
        };
        let asked: Vec<mcp::SaveRequest> = mcp.saves().collect();
        let wires: Vec<mcp::WireRequest> = mcp.wires().collect();
        for request in asked {
            self.save_set(
                engine,
                root,
                Asked::Model,
                request.slot,
                request.id,
                Some(request.reply),
            );
        }
        self.rewire(engine, wires);
    }

    /// Applies requested wire edges to engine state on the current frame.
    ///
    /// Validates edges against deck slot count, updates connections via [`rewired`],
    /// logs the result, and immediately settles MCP reply futures.
    fn rewire(&mut self, engine: &mut Engine, asked: Vec<mcp::WireRequest>) {
        if asked.is_empty() {
            return;
        }
        let mut wires = Vec::with_capacity(asked.len());
        let mut replies = Vec::with_capacity(asked.len());
        for mcp::WireRequest { slot, edge, reply } in asked {
            wires.push((slot, edge));
            replies.push(reply);
        }
        let said = rewired(
            &wires,
            &mut engine.edges,
            &mut engine.aimed,
            engine.deck.slot_count(),
        );
        for (reply, said) in replies.into_iter().zip(said) {
            match &said {
                Ok(line) | Err(line) => println!("{line}"),
            }
            reply.settled(said);
        }
    }

    /// Saves the current configuration of the deck in `slot` to a Set file on a background thread.
    pub(crate) fn save_set(
        &mut self,
        engine: &Engine,
        root: &std::path::Path,
        asked: Asked,
        slot: usize,
        id: Option<String>,
        reply: Option<mcp::Reply>,
    ) {
        // Guard against out-of-range slots before indexing deck state.
        let count = engine.deck.slot_count();
        if !slot_in_range(slot, count) {
            return refused(reply, karakuri_environment::no_such_slot(slot, count));
        }
        let Some(nodes) = self.playing.at(slot) else {
            // **The only way to reach this in this program**: a build landed
            // whose sources the store would not take, which the watcher said at
            // the time. Every slot is seeded at launch, so a slot that has never
            // rebuilt always has an address.
            return refused(
                reply,
                karakuri_environment::nothing_to_save(slot, None, false),
            );
        };
        let sources = setfile::Sources(nodes.iter().map(copied).collect());
        if sources.is_empty() {
            return refused(
                reply,
                karakuri_environment::nothing_to_save(slot, None, true),
            );
        }
        // Validate user-supplied save ID before allocating paths (P-0090, ADR-0292).
        if let Some(said) = id
            .as_deref()
            .and_then(|id| karakuri_mcp::checked_id(id).err())
        {
            println!("keep: {said}");
            return refused(reply, said);
        }
        let id = karakuri_environment::accepted_save(
            slot,
            asked,
            id,
            &sources,
            root,
            reply
                .as_ref()
                .map(|r| r as &dyn karakuri_environment::SaveReply),
        );
        let values = playing_values(
            engine.deck.slot(EngineSlot(slot as u8)).set(),
            &engine.edges,
        );
        let save = Save {
            slot,
            asked,
            id,
            root: root.to_path_buf(),
            sources,
            values,
        };
        let tx = self.save_tx.clone();
        // Spawn background thread to write save without blocking current frame.
        self.in_flight += 1;
        std::thread::spawn(move || {
            let (slot, asked, id) = (save.slot, save.asked, save.id.clone());
            let outcome = save.run();
            let _ = tx.send(Saved {
                slot,
                asked,
                id,
                outcome,
                reply,
            });
        });
    }

    /// Writes a single node procedure into the library on a background worker thread
    /// (Principle 0096, ADR-0338).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn keep_procedure(
        &mut self,
        engine: &Engine,
        root: &std::path::Path,
        asked: Asked,
        slot: usize,
        node: karakuri_operation::NodeAddress,
        id: Option<String>,
        reply: Option<mcp::Reply>,
    ) {
        // **Checked here rather than only where the request came from**, which
        // is `save_set`'s own guard: a press cannot name a slot this deck does
        // not hold and a tool call can.
        let count = engine.deck.slot_count();
        if !slot_in_range(slot, count) {
            return refused(reply, karakuri_environment::no_such_slot(slot, count));
        }
        // **The address as the pane draws it** — `node_addr`'s own spelling,
        // which is the run of text under the operator's eye when they pressed.
        let addr = node_addr(ir_layer(node.layer), node.index);
        let Some(nodes) = self.playing.at(slot) else {
            return refused(
                reply,
                karakuri_environment::nothing_to_save(slot, None, false),
            );
        };
        // Locate matching kept node; built-in nodes or missing addresses lack source procedures (P-0083).
        let Some(found) = nodes.iter().find(|kept| {
            kept.layer == setfile::kind_name(ir_layer(node.layer)) && kept.index == node.index
        }) else {
            let said = format!(
                "`{addr}` on deck {} has no source to keep — the built-in camera is a node with \
                 no procedure behind it, and no other address on this deck is missing one",
                deck_letter(slot as u8)
            );
            println!("keep: {said}");
            return refused(reply, said);
        };
        // **An id an operator typed is checked here**, which is `save_set`'s
        // own wall and its reason: `<name>` becomes one path component, and
        // the console emits what was typed including the empty string.
        if let Some(said) = id
            .as_deref()
            .and_then(|id| karakuri_mcp::checked_id(id).err())
        {
            println!("keep: {said}");
            return refused(reply, said);
        }
        // **A stamp where nobody typed**, which is `accepted_save`'s own
        // convention read one file kind along: the capsule is the press that
        // types nothing (ADR-0128, ADR-0287).
        let name = id.unwrap_or_else(karakuri_environment::history::stamped_id);
        let kept = Kept {
            asked,
            name,
            root: root.to_path_buf(),
            source: found.source.clone(),
            hash: found.hash,
            addr,
            // **Filled by the thread**, and this value is never read: the
            // request and the outcome are one type here because the two carry
            // the same fields, and the `Ok` below is the unwritten state
            // rather than a claim.
            outcome: Ok(std::path::PathBuf::new()),
            reply,
        };
        let tx = self.keep_tx.clone();
        // **A thread per keep**, and detached: no frame waits for it — the
        // save path's own arrangement, and a keep is rarer than a save.
        self.in_flight += 1;
        std::thread::spawn(move || {
            let _ = tx.send(kept.run());
        });
    }

    /// Collects completed procedure save tasks and returns true if any operator keep landed.
    pub(crate) fn finished_keeps(&mut self) -> bool {
        let mut landed = false;
        while let Ok(kept) = self.keeps.try_recv() {
            self.in_flight = self.in_flight.saturating_sub(1);
            let said = kept.said();
            match &said {
                Ok(line) | Err(line) => println!("{line}"),
            }
            landed |= said.is_ok() && kept.asked == Asked::Operator;
            if let Some(reply) = kept.reply {
                reply.settled(said);
            }
        }
        landed
    }

    /// Gathers live deck state and returns a [`Starting`] record used to initialize a recorded session.
    ///
    /// Reads current playing values and sources across all active slots in memory.
    /// The material set for [`HEAD_SLOT`] is saved under `{session}-material` in the library.
    pub(crate) fn head_opening(
        &self,
        engine: &Engine,
        root: &std::path::Path,
        session: &str,
    ) -> Result<Starting, String> {
        let count = engine.deck.slot_count();
        if !slot_in_range(HEAD_SLOT, count) {
            return Err(karakuri_environment::no_such_slot(HEAD_SLOT, count));
        }
        let Some(nodes) = self.playing.at(HEAD_SLOT) else {
            return Err(karakuri_environment::nothing_to_save(
                HEAD_SLOT, None, false,
            ));
        };
        let sources = setfile::Sources(nodes.iter().map(copied).collect());
        if sources.is_empty() {
            return Err(karakuri_environment::nothing_to_save(HEAD_SLOT, None, true));
        }
        let held = session::Held {
            // **What the frame is composited at, and not [`crate::CANVAS`]** —
            // the frame follows the largest enabled output (ADR-0247), so the
            // constant is the size this run *started* at rather than the size
            // it is running at.
            canvas: engine.present.size(),
            look: engine.look,
            master_out: engine.deck.out(),
            master_chain: engine.chain.clone(),
            slots: (0..count)
                .map(|slot| {
                    let at = EngineSlot(slot as u8);
                    session::SlotHeld {
                        nodes: self
                            .playing
                            .at(slot)
                            .map(|nodes| nodes.iter().filter_map(addressed).collect())
                            .unwrap_or_default(),
                        gain: engine.deck.gain(at),
                        opacity: engine.deck.opacity(at),
                        blend: engine.deck.blend(at),
                        // **The request and not the grant.**
                        // `Record::Residency` records what a slot was asked to
                        // do; the governor re-derives the rest on whatever
                        // machine replays it.
                        residency: engine.deck.requested_residency(at),
                        policy: self.slot_policies.policy(slot),
                        mask: engine.deck.mask(at),
                        transport: *engine.deck.transport(at),
                    }
                })
                .collect(),
        };
        // **Slot 0's ride inside the `Save`**, which puts them as it writes the
        // file — see [`setfile::Sources::into_nodes`]. These are the others, and
        // an empty list for slot 0 keeps the index meaning the deck slot.
        let others = (0..count)
            .map(|slot| {
                if slot == HEAD_SLOT {
                    return setfile::Sources(Vec::new());
                }
                setfile::Sources(
                    self.playing
                        .at(slot)
                        .map(|nodes| nodes.iter().map(copied).collect())
                        .unwrap_or_default(),
                )
            })
            .collect();
        Ok(Starting {
            sources: others,
            held,
            material: Save {
                slot: HEAD_SLOT,
                asked: Asked::Operator,
                id: format!("{session}-material"),
                root: root.to_path_buf(),
                sources,
                values: playing_values(
                    engine.deck.slot(EngineSlot(HEAD_SLOT as u8)).set(),
                    &engine.edges,
                ),
            },
        })
    }

    /// Drains and processes saves and sends that completed since the last frame.
    ///
    /// Non-blocking drain called per frame. Returns true if any save completed,
    /// triggering a reload of the library listing.
    pub(crate) fn finished_saves(&mut self) -> bool {
        let mut landed: Vec<Saved> = Vec::new();
        while let Ok(saved) = self.saves.try_recv() {
            landed.push(saved);
        }
        let mut written = false;
        for saved in landed {
            written |= self.took_save(saved);
        }
        // Drain completed sends non-blockingly; sends write outside the store and do not affect library rows.
        while let Ok(sent) = self.sends.try_recv() {
            Keeping::took_send(sent);
        }
        written
    }

    /// Logs the outcome of an export/send operation to the console.
    fn took_send(sent: Sent) {
        println!("{}", sent.said());
    }

    /// Processes a completed save event, logging results and replying to MCP callers if attached.
    fn took_save(&mut self, saved: Saved) -> bool {
        let Saved {
            slot,
            asked,
            id,
            outcome,
            reply,
        } = saved;
        self.in_flight = self.in_flight.saturating_sub(1);
        let (written, said) = match outcome {
            // Operator saves update the library listing; sandbox saves write to sandbox without listing.
            Ok(()) => match asked {
                Asked::Operator => {
                    let said = format!(
                        "  keep: deck {}: saved as set `{id}` — the Library bay's `all` \
                         lists it, and a load off that row puts it back",
                        deck_letter(slot as u8)
                    );
                    println!("{said}");
                    (true, Ok(said))
                }
                Asked::Model => {
                    let said = format!(
                        "  keep: deck {}: saved as set `{id}` in the sandbox — \
                         `<store>/{}/{id}{}`. A save asked for over MCP is kept there \
                         rather than in the operator's library, so `all` does not list \
                         it and no load off that row reaches it; the operator's own `k` writes \
                         library",
                        deck_letter(slot as u8),
                        karakuri_store::store::Store::SANDBOX,
                        karakuri_store::store::Store::SET_FILE_SUFFIX,
                    );
                    println!("{said}");
                    (false, Ok(said))
                }
            },
            Err(e) => {
                let said = format!(
                    "  keep: deck {}: set `{id}` was not saved: {e}",
                    deck_letter(slot as u8)
                );
                println!("{said}");
                (false, Err(said))
            }
        };
        if let Some(reply) = reply {
            reply.settled(said);
        }
        written
    }

    /// Updates node tracking for a slot following a build swap and returns changed `(layer, index)` nodes.
    ///
    /// Drains `built` receiver, updates `playing` node records, and computes the diff
    /// of added or modified nodes for staging lane presentation (ADR-0326).
    pub(crate) fn took_up(
        &mut self,
        aims: &[Aiming],
        slot: usize,
        landed: u64,
    ) -> Vec<(&'static str, u32)> {
        let mut ready: Vec<watch::Built> = Vec::new();
        while let Ok(built) = self.built.try_recv() {
            ready.push(built);
        }
        for built in ready {
            self.pending.push(built);
        }
        let at = self.pending.iter().position(|built| built.id == landed);
        let built = at.map(|at| self.pending.remove(at));
        let nodes = built
            .as_ref()
            .zip(aims.get(slot))
            .map(|(built, aiming)| built_nodes(built, &aiming.at));
        // Compute diff against previously playing nodes by layer, index, and content hash.
        let changed = match (self.playing.at(slot), nodes.as_ref()) {
            (Some(was), Some(now)) => now
                .iter()
                .filter(|node| {
                    !was.iter().any(|before| {
                        before.layer == node.layer
                            && before.index == node.index
                            && before.hash == node.hash
                    })
                })
                .map(|node| (node.layer, node.index))
                .collect(),
            _ => Vec::new(),
        };
        self.playing.landed(slot, nodes);
        changed
    }

    /// Blocks up to [`karakuri_environment::SAVE_WAIT`] on shutdown to drain in-flight saves and keeps.
    pub(crate) fn awaited_saves(&mut self) {
        self.finished_saves();
        self.finished_keeps();
        if self.in_flight == 0 {
            return;
        }
        println!(
            "waiting up to {:.0}s for {} save{} still being written",
            karakuri_environment::SAVE_WAIT.as_secs_f32(),
            self.in_flight,
            match self.in_flight {
                1 => "",
                _ => "s",
            }
        );
        let deadline = Instant::now() + karakuri_environment::SAVE_WAIT;
        while self.in_flight > 0 {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            // Poll saves channel with bounded timeout and drain keeps.
            let slice = left.min(std::time::Duration::from_millis(20));
            if let Ok(saved) = self.saves.recv_timeout(slice) {
                self.took_save(saved);
            }
            self.finished_keeps();
        }
        if self.in_flight > 0 {
            println!(
                "  {} save{} still unfinished after {:.0}s — each is written or it is not, and \
                 nothing here claims either way",
                self.in_flight,
                match self.in_flight {
                    1 => "",
                    _ => "s",
                },
                karakuri_environment::SAVE_WAIT.as_secs_f32(),
            );
        }
    }
}
