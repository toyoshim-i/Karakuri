use std::sync::mpsc;

use karakuri_ir::Kind;
#[cfg(test)]
pub(crate) use karakuri_operation::gate;
pub(crate) use karakuri_operation::{InputPort, NodeAddress, Operation};

use crate::tools::OperateRequest;

/// Render loop events communicated back to the MCP server.
pub enum Event {
    /// A build landed, was overloaded, or failed with status message.
    Swap { slot: usize, said: String },
}

/// Request sent to the render loop to save a slot's current live Set.
pub struct SaveRequest {
    /// Slot index to save.
    pub slot: usize,
    /// Optional target filename ID. Defaults to timestamp-based name if None.
    pub id: Option<String>,
    /// Reply channel.
    pub reply: Reply,
}

/// Channel sender used by the render loop to reply to a request.
pub struct Reply(pub(crate) mpsc::Sender<News>);

impl Reply {
    /// Signals that the render loop accepted the request.
    pub fn accepted(&self, said: &str) {
        let _ = self.0.send(News::Accepted(said.to_string()));
    }

    /// Signals the final outcome of the request.
    pub fn settled(self, said: Result<String, String>) {
        let _ = self.0.send(News::Settled(said));
    }
}

impl karakuri_environment::SaveReply for Reply {
    fn accepted(&self, said: &str) {
        Reply::accepted(self, said)
    }
}

/// Request to rewire an input edge on the render loop.
pub struct WireRequest {
    pub slot: usize,
    pub edge: karakuri_engine::set::Edge,
    pub reply: Reply,
}

/// Internal notification items delivered over a `Reply` channel.
pub(crate) enum News {
    Accepted(String),
    Settled(Result<String, String>),
}

/// Communication handle held by the render loop to receive requests and report events.
pub struct Reporter {
    pub(crate) sender: mpsc::SyncSender<Event>,
    pub(crate) requests: mpsc::Receiver<SaveRequest>,
    pub(crate) wires: mpsc::Receiver<WireRequest>,
    pub(crate) operations: mpsc::Receiver<OperateRequest>,
    pub(crate) dropped: std::sync::Arc<std::sync::atomic::AtomicU64>,
    pub(crate) port: u16,
}

impl Reporter {
    /// Reports a swap event from the render loop. Non-blocking.
    pub fn swap(&self, slot: usize, said: &str) {
        let event = Event::Swap {
            slot,
            said: said.to_string(),
        };
        if self.sender.try_send(event).is_err() {
            self.dropped
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Drains pending save requests without blocking.
    pub fn saves(&self) -> impl Iterator<Item = SaveRequest> + '_ {
        self.requests.try_iter()
    }

    /// Drains pending wire requests without blocking.
    pub fn wires(&self) -> impl Iterator<Item = WireRequest> + '_ {
        self.wires.try_iter()
    }

    /// Drains pending operate requests without blocking.
    pub fn operations(&self) -> impl Iterator<Item = OperateRequest> + '_ {
        self.operations.try_iter()
    }

    /// Returns the bound server port.
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// Thread-safe registry mapping deck slots to their procedural backing files.
#[derive(Clone, Default)]
pub struct Slots(std::sync::Arc<std::sync::RwLock<Vec<Pointed>>>);

/// Target paths for a slot: head procedure path and rest procedure paths.
pub type Pointed = (std::path::PathBuf, Vec<std::path::PathBuf>);

impl Slots {
    /// The deck as it stands, one pair per slot in slot order.
    ///
    /// What a run seeds this with is where its watchers are pointed when it is
    /// made, and a run whose slots never move — `karakuri-cli`, which loads nothing
    /// mid-run — keeps that value for the whole run and calls nothing else here.
    pub fn of(pairs: Vec<Pointed>) -> Slots {
        Slots(std::sync::Arc::new(std::sync::RwLock::new(pairs)))
    }

    /// A deck nothing is published to, for a harness that builds an engine and
    /// serves nothing.
    ///
    /// It holds no row, so [`Slots::re_point`] writes nothing into it and every
    /// address it could be asked about is refused with what it holds — which is
    /// zero slots. It cannot reach a client: [`serve`] refuses a handle with no
    /// slots in it before it binds, which is the same refusal a run given no
    /// procedure files gets.
    pub fn unpointed() -> Slots {
        Slots::of(Vec::new())
    }

    /// One slot is pointed somewhere else, said by whoever moved it.
    ///
    /// Called where an aim is *sent* — a library load, and a rewiring that restates
    /// one — so that the published layout cannot be a step behind the watcher's.
    /// The pair is derived from the aim here rather than by the caller, because a
    /// caller deriving it would be the second derivation of *what is this slot
    /// running* that [`Slots`]' own head is about.
    ///
    /// A slot this handle does not hold is not written, and nothing is grown to
    /// make room: the row count is the deck's, settled when the run made the
    /// handle, and a handle with no row for this slot is one no server and no
    /// landing is reading — a harness that built an engine and served nothing.
    ///
    /// A poisoned lock is recovered rather than dropped, which is where this parts
    /// company with [`crate::Opening::set`]. What is behind that lock is a whole
    /// pair per slot written in one assignment, so a panic elsewhere cannot have
    /// left half of one; and the safe answer there — a closed class — has no
    /// counterpart here, because *the layout before the load* is exactly the wrong
    /// answer this type exists to stop giving.
    pub fn re_point(&self, slot: usize, at: &karakuri_environment::watch::Aim) {
        let mut held = self.0.write().unwrap_or_else(|held| held.into_inner());
        if let Some(pair) = held.get_mut(slot) {
            *pair = (
                at.head.path.clone(),
                at.rest.iter().map(|node| node.path.clone()).collect(),
            );
        }
    }

    /// How many slots this deck holds, which is what every refusal about a slot
    /// number is measured against ([`crate::no_such_slot`]).
    pub fn count(&self) -> usize {
        self.held().len()
    }

    /// What is published now. Read on every call and never held across one, for
    /// [`crate::Opening::read`]'s reason said about a layout: a slot the operator
    /// loaded a Set onto between two calls has moved for the second.
    fn held(&self) -> std::sync::RwLockReadGuard<'_, Vec<Pointed>> {
        self.0.read().unwrap_or_else(|held| held.into_inner())
    }

    /// Returns a slot's files categorized by their declared `Kind` layer and index.
    pub(crate) fn nodes(
        &self,
        slot: usize,
    ) -> Result<Vec<(Kind, usize, std::path::PathBuf)>, String> {
        let held = self.held();
        let pair = held
            .get(slot)
            // **The one sentence, from [`crate::no_such_slot`].** This used to
            // be its own spelling — `this deck holds 0-3` against the keys'
            // `this deck holds slots 0-3` — so a model calling `save_set` and
            // an operator pressing a digit were told the same mistake in
            // different words about the same control. It also handled the empty
            // deck for its own reason, which that function has too: `len() - 1`
            // underflowed on an empty deck and took the whole surface with it,
            // the panic unwinding out of the listener thread so that a process
            // which had announced a port was silently no longer on it. `serve`
            // refuses an empty deck now; this stays correct anyway.
            .ok_or_else(|| karakuri_environment::no_such_slot(slot, held.len()))?;
        // **The head's own `kind` line, and `L1` only where it has none.**
        // That fallback is `history::seed`'s, exactly: the first path of a
        // chain with no `kind` in it falls back to L1 and every later one to
        // L4, so a file this scan cannot read is addressed here under the layer
        // its snapshots are filed under there.
        let head = std::fs::read(&pair.0)
            .ok()
            .and_then(|source| karakuri_environment::history::declared_kind(&source))
            .and_then(layer_named)
            .unwrap_or(Kind::L1);
        let mut nodes = vec![(head, 0, pair.0.clone())];
        // The next free index per layer, which the head has already taken one
        // of: a `--set` chain naming a second `kind L1` is a second source, and
        // it is L1 number 1 rather than the beginning of a fresh count.
        let mut next: Vec<(Kind, usize)> = vec![(head, 1)];
        for path in &pair.1 {
            let layer = std::fs::read(path)
                .ok()
                .and_then(|source| karakuri_environment::history::declared_kind(&source))
                .and_then(layer_named)
                .unwrap_or(Kind::L4);
            let index = match next.iter_mut().find(|(held, _)| *held == layer) {
                Some((_, free)) => {
                    let index = *free;
                    *free += 1;
                    index
                }
                None => {
                    next.push((layer, 1));
                    0
                }
            };
            nodes.push((layer, index, path.clone()));
        }
        Ok(nodes)
    }

    /// Checks whether `slot` is within bounds without reading files from disk.
    pub(crate) fn holds(&self, slot: usize) -> Result<(), String> {
        let count = self.count();
        if slot < count {
            Ok(())
        } else {
            Err(karakuri_environment::no_such_slot(slot, count))
        }
    }

    /// Resolves an address string layer to a file path.
    pub fn file(
        &self,
        slot: usize,
        layer: &str,
        index: usize,
    ) -> Result<std::path::PathBuf, String> {
        let Some(kind) = layer_named(layer) else {
            return Err(format!(
                "`{layer}` is not a layer: {}",
                LAYERS
                    .iter()
                    .map(|kind| layer_name(*kind))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        };
        self.path(slot, kind, index)
    }

    /// The file one `(slot, layer, index)` address names.
    pub fn path(
        &self,
        slot: usize,
        layer: Kind,
        index: usize,
    ) -> Result<std::path::PathBuf, String> {
        let nodes = self.nodes(slot)?;
        if let Some((_, _, path)) = nodes.iter().find(|(l, i, _)| *l == layer && *i == index) {
            return Ok(path.clone());
        }
        // **What the slot holds, rather than "no such node".** An index past
        // the end and a layer this slot does not use are different mistakes,
        // and a model told which one it made can fix its own call — the same
        // reason the checker's diagnostics come back through here instead of
        // going to a terminal nobody is watching.
        let name = layer_name(layer);
        Err(match nodes.iter().filter(|(l, _, _)| *l == layer).count() {
            0 => format!("slot {slot} holds no {name}: {}", absent(layer)),
            1 => format!("slot {slot} holds one {name} and `index` is {index}"),
            n => format!(
                "slot {slot} holds {n} {name} nodes, so `index` is 0-{}",
                n - 1
            ),
        })
    }
}

/// Every layer a slot's files can be on, in the order they compose.
/// The set of layers a slot can hold.
pub(crate) const LAYERS: [Kind; 5] = [Kind::L1, Kind::L2, Kind::L3, Kind::L4, Kind::Field];

/// Parses a case-insensitive layer name into a `Kind`.
pub(crate) fn layer_named(name: &str) -> Option<Kind> {
    Some(match name.to_ascii_uppercase().as_str() {
        "L1" => Kind::L1,
        "L2" => Kind::L2,
        "L3" => Kind::L3,
        "L4" => Kind::L4,
        "FIELD" => Kind::Field,
        _ => return None,
    })
}

/// The name back again, for a schema and for a sentence.
///
/// Exhaustive on purpose: a sixth `Kind` should not compile until somebody has
/// decided what this surface calls it and whether [`LAYERS`] offers it.
pub(crate) fn layer_name(layer: Kind) -> &'static str {
    match layer {
        Kind::L1 => "L1",
        Kind::L2 => "L2",
        Kind::L3 => "L3",
        Kind::L4 => "L4",
        Kind::Field => "Field",
        Kind::L5 => "L5",
    }
}

/// The compiler's layer, in the vocabulary's spelling.
///
/// One function per list, in the one package that depends on both — which is
/// what `karakuri-operation`'s module documentation prescribes for every list
/// it copies, and what `mix::blend_mode` and its three neighbours already are
/// for the mix. Exhaustive both ways round, so a sixth `Kind` or a sixth
/// `karakuri_operation::Layer` stops the build here until somebody has said
/// what the other one calls it.
///
/// [`NodeAddress`]'s first caller in this workspace is this surface, and that
/// is not an accident: the manual's own gap section says MIDI *"cannot express
/// a node address, a parameter name or an id"*, a key press has nothing to say
/// one with, and the panel does not reach inside a Set. A node address is the
/// thing MCP can say and the other three cannot.
pub(crate) fn layer_of(layer: Kind) -> karakuri_operation::Layer {
    karakuri_environment::meta::op_layer_of(layer)
}

/// And back, for the two things that want the compiler's own: resolving an
/// address to a file, and comparing a written source's `kind` line against the
/// address it arrived at.
pub(crate) fn kind_of(layer: karakuri_operation::Layer) -> Kind {
    karakuri_environment::meta::kind_of_op(layer)
}

/// The layers, as a client is told them in a refusal.
pub(crate) fn layer_list() -> String {
    LAYERS
        .iter()
        .map(|layer| layer_name(*layer))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What it means for a slot to hold none of a layer, which is a different thing
/// for each of them: three are optional and two cannot be missing.
pub(crate) fn absent(layer: Kind) -> &'static str {
    match layer {
        // Reachable now that the head is filed under the `kind` it declares:
        // a chain of nothing but a deformation and a renderer holds no
        // geometry, and this is what such a slot is told. It does not get as
        // far as a frame — the build refuses a Set with no L1 — but this
        // surface answers before anything is built.
        Kind::L1 => {
            "a Set needs a geometry, and every file this slot names declares some other layer"
        }
        Kind::L2 => {
            "a deformation is optional, and one is added by naming its file in the same \
             `--set` chain"
        }
        Kind::L3 => "a camera is optional, and a slot without one looks from the built-in orbit",
        Kind::L4 => "a Set needs at least one renderer",
        Kind::Field => {
            "a `kind Field` is optional, and is code the other procedures evaluate rather \
             than a node of its own"
        }
        // **Every slot holds none, and will while the chain is fixed.** A frame
        // effect runs in the master chain rather than in a Set, and the chain is
        // still three hand-written passes — so this is what a slot is told
        // about a kind that compiles and has nowhere to be placed yet.
        Kind::L5 => {
            "a frame effect runs in the master chain rather than in a Set, and the chain is \
             still three fixed passes"
        }
    }
}

/// Maximum allowed request body size (1 MB).
pub(crate) const MAX_BODY: usize = 1 << 20;

/// Inactivity timeout before closing a connection.
pub(crate) const IDLE: std::time::Duration = std::time::Duration::from_secs(30);

/// Maximum queued swap events before older ones are dropped.
pub(crate) const QUEUED: usize = 256;

/// Channel capacity for render loop request queues.
pub(crate) const ASKED: usize = 16;

/// Maximum length in bytes for Set identifiers.
pub(crate) const MAX_ID: usize = 64;

/// Regex pattern matching valid Set identifiers.
pub(crate) const ID_PATTERN: &str = "^[A-Za-z0-9_-]+$";

/// Timeout waiting for a save operation outcome.
pub(crate) const SAVE_REPLY: std::time::Duration =
    std::time::Duration::from_secs(karakuri_environment::SAVE_WAIT.as_secs() + 5);

/// Timeout waiting for an input wire operation outcome.
pub(crate) const WIRE_REPLY: std::time::Duration = std::time::Duration::from_secs(5);

/// Server runtime state shared across requests.
pub(crate) struct State {
    pub(crate) slots: Slots,
    pub(crate) store: std::path::PathBuf,
    pub(crate) watching: bool,
    pub(crate) opening: karakuri_environment::Opening,
    pub(crate) slot_policies: karakuri_environment::SlotPolicies,
    pub(crate) events: mpsc::Receiver<Event>,
    pub(crate) asked: mpsc::SyncSender<SaveRequest>,
    pub(crate) wiring: mpsc::SyncSender<WireRequest>,
    pub(crate) operating: mpsc::SyncSender<OperateRequest>,
    pub(crate) recent: Vec<String>,
    pub(crate) dropped: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

/// Number of recent swap events kept in memory.
pub(crate) const RECENT: usize = 32;

impl State {
    pub(crate) fn drain(&mut self) {
        while let Ok(Event::Swap { slot, said }) = self.events.try_recv() {
            self.recent.push(format!("slot {slot}: {said}"));
        }
        if self.recent.len() > RECENT {
            self.recent.drain(..self.recent.len() - RECENT);
        }
    }
}
