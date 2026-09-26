use super::*;

/// Generates and writes session head metadata and Set files to allow deterministic replay.
///
/// Combines slot 0 Set representation with current deck procedures and mixer state
/// (see `karakuri_environment::session::head`).
pub(crate) fn session_head(
    args: &Args,
    placed: &[Vec<Placed>],
    l1s: &[karakuri_ir::typed::Checked],
    store: &karakuri_store::store::Store,
    id: &str,
) -> Vec<karakuri_store::ndjson::Line> {
    if let Some(set) = &args.load_set {
        return match store.read_set(set) {
            Ok(lines) => lines,
            Err(e) => {
                eprintln!("karakuri-cli: reading set `{set}` for the session's head: {e}");
                std::process::exit(2);
            }
        };
    }
    let Some(nodes) = placed.first().filter(|nodes| !nodes.is_empty()) else {
        eprintln!("karakuri-cli: nothing to record — no Set to put at the session's head");
        std::process::exit(2);
    };
    let material = format!("{id}-material");
    let camera = karakuri_engine::camera::Orbit::default();
    // **Every source into the store before the file that references them.**
    // The writer takes hashes now — see `setfile::Node` — and this is the
    // caller whose paths are still exactly what the run compiled a moment ago.
    let nodes = match saving_nodes(store, nodes) {
        Ok(nodes) => nodes,
        Err(e) => {
            eprintln!("karakuri-cli: writing the session's material: {e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = setfile::save(
        store,
        // **The operator's**: this is the material of a run they started, kept
        // where they will look for it.
        Asked::Operator,
        &material,
        setfile::Saving {
            nodes: &nodes,
            capacities: &saving_capacities(args, l1s),
            params: &args.overrides,
            bindings: &args.bindings,
            // Which node fills each declared slot.
            edges: &args.edges,
            camera: &camera,
            // Evaluates layering from flags and defaults for initial material record.
            layering: layering_for(args, 0, recorded_layering(args, 0)),
            live: None,
            seeds: &saving_seeds(args, l1s),
        },
    ) {
        eprintln!("karakuri-cli: writing the session's material: {e}");
        std::process::exit(2);
    }
    match store.read_set(&material) {
        Ok(lines) => lines,
        Err(e) => {
            eprintln!("karakuri-cli: reading back the session's material: {e}");
            std::process::exit(2);
        }
    }
}

/// Captures active deck configuration and residency state at the start of a session.
pub(crate) fn held_deck(
    args: &Args,
    placed: &[Vec<Placed>],
    deck: &karakuri_engine::Deck,
    canvas: (u32, u32),
) -> session::Held {
    session::Held {
        canvas,
        look: args.look,
        master_out: deck.out(),
        master_chain: Vec::new(),
        slots: (0..deck.slot_count())
            .map(|slot| {
                let at = EngineSlot(slot as u8);
                session::SlotHeld {
                    nodes: placed
                        .get(slot)
                        .map(|nodes| {
                            nodes
                                .iter()
                                .map(|node| {
                                    (
                                        karakuri_environment::meta::layer_of(node.layer),
                                        node.index,
                                        node.hash(),
                                    )
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    gain: deck.gain(at),
                    opacity: deck.opacity(at),
                    blend: deck.blend(at),
                    // Records requested residency; effective level is re-governed on replay.
                    residency: deck.requested_residency(at),
                    policy: karakuri_operation::SlotPolicy::Auto,
                    mask: deck.mask(at),
                    transport: *deck.transport(at),
                }
            })
            .collect(),
    }
}

/// Seeds the store with startup source artifacts for all slots when recording a session.
///
/// Ensures all referenced procedure source hashes in the recorded session are resolvable on replay.
pub(crate) fn seed_store_for_replay(store: &karakuri_store::store::Store, placed: &[Vec<Placed>]) {
    for (slot, nodes) in placed.iter().enumerate() {
        for node in nodes {
            if let Err(e) = node.put(store) {
                eprintln!(
                    "  slot {slot}: {e} — a record naming what this slot launched with \
                     will name a source this session's replay cannot resolve"
                );
            }
        }
    }
}

/// Resolves active procedure hashes and sources for a slot to be saved.
pub(crate) fn live_sources(playing: Option<&Nodes>, startup: &[Placed]) -> Sources {
    Sources(playing.map_or_else(Vec::new, |nodes| {
        nodes
            .iter()
            .enumerate()
            .map(|(at, (layer, index, hash))| {
                let placed = startup.get(at);
                // Include source and metadata when the active node matches the startup hash;
                // dynamically loaded nodes are already persisted by the watcher.
                let (source, meta) = placed
                    .filter(|p| p.hash() == *hash)
                    .map(|p| {
                        (
                            std::sync::Arc::clone(&p.source),
                            std::sync::Arc::clone(&p.meta),
                        )
                    })
                    .unzip();
                SavedNode {
                    layer,
                    index: *index,
                    hash: *hash,
                    name: placed.and_then(|p| p.named.name.clone()),
                    source,
                    meta,
                }
            })
            .collect()
    }))
}

/// Prepares node records for saving and writes corresponding source artifacts to store.
pub(crate) fn saving_nodes(
    store: &karakuri_store::store::Store,
    placed: &[Placed],
) -> Result<Vec<setfile::Node>, String> {
    placed
        .iter()
        .map(|node| node.put(store).map(|_| node.node()))
        .collect()
}

/// Computes actual operating capacities for each geometry in the Set to be saved.
///
/// Respects flag overrides or procedure declared defaults (ADR-0066).
pub(crate) fn saving_capacities(args: &Args, l1s: &[karakuri_ir::typed::Checked]) -> Vec<u32> {
    l1s.iter().map(|l1| capacity_for(args, l1)).collect()
}

/// Computes salt seeds for geometries in slot 0 to be recorded in the Set file.
pub(crate) fn saving_seeds(args: &Args, l1s: &[karakuri_ir::typed::Checked]) -> Vec<u32> {
    salts_for(seed_for(0), recorded_salts(args, 0), l1s.len())
}

/// Saves slot 0 as a Set file into the store.
pub(crate) fn save_set(
    args: &Args,
    placed: &[Vec<Placed>],
    l1s: &[karakuri_ir::typed::Checked],
    id: &str,
) {
    let store = open_store(args);
    let Some(nodes) = placed.first().filter(|nodes| !nodes.is_empty()) else {
        eprintln!("karakuri-cli: --save-set needs a `.kir` chain to save");
        std::process::exit(1);
    };
    if placed.len() > 1 {
        eprintln!(
            "  only slot 0 is saved: a Set file describes one Set, and which Sets a deck \
             is holding belongs to a session"
        );
    }
    let camera = karakuri_engine::camera::Orbit::default();
    let nodes = match saving_nodes(&store, nodes) {
        Ok(nodes) => nodes,
        Err(e) => {
            eprintln!("karakuri-cli: {e}");
            std::process::exit(1);
        }
    };
    match setfile::save(
        &store,
        Asked::Operator,
        id,
        setfile::Saving {
            nodes: &nodes,
            capacities: &saving_capacities(args, l1s),
            params: &args.overrides,
            bindings: &args.bindings,
            edges: &args.edges,
            camera: &camera,
            layering: layering_for(args, 0, recorded_layering(args, 0)),
            live: None,
            seeds: &saving_seeds(args, l1s),
        },
    ) {
        Ok(()) => eprintln!(
            "wrote set `{id}` to {} — load it with `--load-set {id}`",
            args.store.display()
        ),
        Err(e) => {
            eprintln!("karakuri-cli: {e}");
            std::process::exit(1);
        }
    }
}

/// Loads a Set file and merges its definitions with CLI arguments.
pub(crate) fn load_set(args: &mut Args, id: &str) -> setfile::Loaded {
    let store = open_store(args);
    let loaded = match setfile::load(&store, id) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("karakuri-cli: {e}");
            std::process::exit(1);
        }
    };
    for note in &loaded.notes {
        eprintln!("  {note}");
    }
    // Propagate recorded capacity to CLI args.
    if let Some(capacity) = loaded.capacities.first().copied().flatten() {
        args.capacity = capacity;
        args.capacity_given = true;
    }
    let mut overrides = loaded.params.clone();
    overrides.append(&mut args.overrides);
    args.overrides = overrides;
    let mut bindings = loaded.bindings.clone();
    bindings.append(&mut args.bindings);
    args.bindings = bindings;
    // Retain file edges not explicitly overridden on the command line.
    let mut edges = loaded.edges.clone();
    edges.retain(|e| {
        !args
            .edges
            .iter()
            .any(|given| given.node == e.node && given.slot == e.slot)
    });
    edges.append(&mut args.edges);
    args.edges = edges;
    args.from_set = Some(FromSet {
        salts: loaded.salts.clone(),
        camera: loaded.camera,
        layering: loaded.layering,
        live: loaded.live,
        capacities: loaded.capacities.clone(),
    });
    loaded
}

/// Drains completed background saves from the receiver until all arrive or deadline passes.
pub(crate) fn drained_saves(
    rx: &std::sync::mpsc::Receiver<Saved>,
    in_flight: usize,
    deadline: Instant,
) -> Vec<Saved> {
    let mut landed = Vec::new();
    while landed.len() < in_flight {
        let Some(left) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        match rx.recv_timeout(left) {
            Ok(saved) => landed.push(saved),
            Err(_) => break,
        }
    }
    landed
}

/// Background task payload for writing a Set file to the store.
pub(crate) struct Save {
    pub(crate) slot: usize,
    pub(crate) asked: Asked,
    pub(crate) id: String,
    pub(crate) root: PathBuf,
    pub(crate) sources: Sources,
    pub(crate) values: setfile::Owned,
}

impl Save {
    /// Writes the Set file and referenced procedure source artifacts off-thread.
    pub(crate) fn run(self) -> Result<(), String> {
        let Save {
            asked,
            id,
            root,
            sources,
            mut values,
            ..
        } = self;
        let store = karakuri_store::store::Store::open(&root)
            .map_err(|e| format!("store `{}`: {e}", root.display()))?;
        values.nodes = sources.into_nodes(&store)?;
        setfile::save(&store, asked, &id, values.saving())
    }
}

/// Completion report of an asynchronous save operation.
pub(crate) struct Saved {
    pub(crate) slot: usize,
    pub(crate) asked: Asked,
    pub(crate) id: String,
    pub(crate) outcome: Result<(), String>,
    pub(crate) reply: Option<mcp::Reply>,
}

/// Emits a refusal message to stderr and resolves any pending client reply.
pub(crate) fn refused(reply: Option<mcp::Reply>, said: String) {
    eprintln!("{said}");
    if let Some(reply) = reply {
        reply.settled(Err(said));
    }
}
