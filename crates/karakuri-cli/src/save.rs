use super::*;

/// **The material a session carries at its head**, so that a replay can build
/// what the run was playing.
///
/// This used to be the `--load-set` file or nothing, and "or nothing" was a
/// hole in the invariant the whole record stream exists for: recording without
/// `--load-set` wrote a timeline of ticks with no material under it, and
/// `--replay` refused it with "has no L1 slot" long after the set was over.
/// Nothing said so at the time.
///
/// So the head is written from **what the run is actually playing**. Without a
/// Set file there is one to make: the `.kir` pair, the capacity, the params,
/// the bindings and the seed are exactly what `--save-set` writes, and putting
/// the sources in the store is what makes the head's hashes resolve on the way
/// back. The Set file it leaves behind is named after the session, so a
/// recording is also a saved Set and neither had to be asked for twice.
///
/// **Only slot 0's material, and it says so**, which is the limitation
/// underneath rather than a choice made here: a Set file describes one Set and
/// a session stream has no way to say what a *deck* held. Everything else about
/// the performance is recorded per slot — gain, blend, residency, preview — so
/// a multi-slot session replays those against a deck of one and reports the
/// rest. Closing it is a format change, and it is named in `docs/ir-spec.md`
/// where the records are.
pub(crate) fn session_head(
    args: &Args,
    placed: &[Vec<Placed>],
    l1s: &[karakuri_ir::typed::Checked],
    store: &karakuri_store::store::Store,
    id: &str,
) -> Vec<karakuri_store::ndjson::Line> {
    if placed.len() > 1 {
        eprintln!(
            "  only slot 0's material is in the session's head — a session stream cannot \
             say what a deck held, so the other {} will not replay",
            placed.len() - 1
        );
    }
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
            // **Which node fills each declared slot**, saved so the file
            // rebuilds: a slot nothing binds is refused where the Set is built,
            // so a Set file that dropped its edges would be one that no longer
            // loads.
            edges: &args.edges,
            camera: &camera,
            // **The flags', on the terms every other value here is theirs.**
            // This head is written before the first frame, from the material
            // the run was started with — there is no Set to read a layering
            // off yet — and a session whose slot 0 composites has to say so or
            // the `select` records it goes on to write land on a replay with
            // no fold to select in. The other branch above needs none of this:
            // a run started from `--load-set` copies that file's lines
            // verbatim, `merge` among them.
            layering: layering_for(args, 0, recorded_layering(args, 0)),
            // **Nothing yet, and it is not an omission.** A selection is made
            // with `r` during a performance, so at the instant a head is
            // written there is none — and one made later is a `select` record
            // in the stream, which replays where it happened rather than
            // before the first frame.
            live: None,
            seeds: &saving_seeds(args, l1s),
        },
    ) {
        // Fatal, on the same terms the recorder itself is: `--record-session`
        // was asked for, and a run that continued would be a performance
        // nobody can replay with nothing saying so.
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

/// **Every slot's launch sources into the store, because this run records one.**
///
/// A `procedure` record naming a slot's launch hashes is resolved by a replay
/// reading the artifact back, and a record naming bytes nobody kept is the same
/// silence as no record at all, so a recorded run owes the store those bytes
/// before the first frame.
///
/// **What used to name them is gone, and this is left standing rather than
/// removed here** (ADR-0316). The reader was a rollback onto the launch
/// version: the engine put a previous Set back and the stream had to say so, in
/// records naming hashes only this seeding had put anywhere. Nothing puts a
/// version back now — a version over the budget stays in the slot, stopped —
/// so no record names a launch hash that a save has not also stored. Whether a
/// recorded run still owes these bytes is a decision about the record
/// vocabulary and is the maintainer's; taking them away on this function's own
/// authority would be a replay that resolves nothing, discovered later.
///
/// **And only a recorded run owes them**, which is the judgement this function
/// exists to hold. The seeding used to happen for every windowed run on exactly
/// this reasoning, and the reasoning does not reach that far: a run with no
/// recorder names no hash anywhere outside itself, and a save that does name
/// one puts its own bytes as it writes the file — see [`Sources::into_nodes`].
/// What the wider version cost was a `.karakuri` directory created by a plain
/// windowed run, which `docs/manual.md` promises does not happen.
///
/// [`session_head`] has already put slot 0's material here on its way past;
/// `put_artifact` is content-addressed, so this repeats nothing and exists for
/// the other slots, which a session head cannot describe but a `procedure`
/// record can still name.
///
/// **Reported and not fatal.** The recorder itself is fatal on failure because
/// a run that continued would be a performance nobody can replay with nothing
/// saying so; this is narrower — one slot's launch material would be
/// unresolvable — and the sentence is the saying.
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

/// **Where one slot's sources come from at save time**: the hashes of what it is
/// running, and nothing else.
///
/// A slot is running a version whose bytes need not be on disk under any name.
/// An edit that fails to compile stays on disk untouched, and a run without
/// `--watch` never picks a file up at all — so the path and the picture can
/// disagree in ordinary ways, and in each of them a save that re-read the path
/// would write down a version nobody had seen. The hash is what still
/// points at what is on screen, which is why it is the *only* thing this reads
/// and why every slot has one from launch — see [`Running::at_launch`]. The
/// bytes behind a launch hash travel with it, because on a run that has saved
/// nothing they exist nowhere else; see [`SavedNode::source`].
///
/// `None` is a slot with no address to name: one filled straight from a Set file
/// with nothing to watch it, or one that has just taken a build whose sources
/// could not be stored and said so at the time. It saves nothing rather than
/// guessing, and `Live::save_set` says which it was.
///
/// **The operator's names are zipped on by position.** Both lists are in the
/// order the files were spelled — `sort_slot` keeps that deliberately and the
/// watcher zips its hashes onto the same list — so entry `n` of one is entry
/// `n` of the other. A name cannot come from the hashes: it belongs to the
/// *use* rather than to the procedure, so nothing a `procedure` record carries
/// could hold it.
///
/// A free function rather than a method, so that the choice — which is the
/// whole of what a live save gets right or wrong about what is on screen — can
/// be checked without a window and a GPU.
pub(crate) fn live_sources(playing: Option<&Nodes>, startup: &[Placed]) -> Sources {
    Sources(playing.map_or_else(Vec::new, |nodes| {
        nodes
            .iter()
            .enumerate()
            .map(|(at, (layer, index, hash))| {
                let placed = startup.get(at);
                // **Carried only where the address says these are the bytes on
                // screen.** Equal hashes mean the slot is still running what it
                // launched with at this node, so the compiled text this process
                // is holding is what the file will reference and the store has
                // to be given it. Unequal means a build put that version there,
                // and the watcher stored it as it built it — there is nothing
                // here to add.
                //
                // **One predicate asked once, yielding the pair.** The card and
                // the bytes travel on exactly the same condition, and
                // `SavedNode::meta` states that as an invariant —
                // `Sources::into_nodes` writes the card inside the `if let` for
                // the source and would silently drop a card that outlived its
                // bytes. Asked twice it was two derivations of one question with
                // nothing holding them together, which is the defect this file
                // has already paid for in `Running` and in `Sources`.
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

/// One slot's nodes as the records they will be written as, with every source
/// put in the store first — which is what makes the hashes the file references
/// resolve on the way back in.
pub(crate) fn saving_nodes(
    store: &karakuri_store::store::Store,
    placed: &[Placed],
) -> Result<Vec<setfile::Node>, String> {
    placed
        .iter()
        .map(|node| node.put(store).map(|_| node.node()))
        .collect()
}

/// What a saved Set says each of its geometries runs at: **what the run was
/// actually drawing**.
///
/// This wrote `args.capacity` — the flag's number, or its default when no flag
/// was given — where the run itself asks [`capacity_for`], which prefers the
/// procedure's own declared default. So saving `lattice_shell` recorded 262144
/// and the run that saved it drew 32768, and loading the file back gave a
/// visibly different picture: a larger, smeared lattice.
///
/// **That falsified the one promise the format makes** — a run driven by the
/// file renders the same frame as the run whose flags wrote it
/// (`docs/adr/0066-a-flag-becomes-a-record-writer.md`). It was invisible while a Set was a pair, because the number was wrong in
/// the file and wrong again on the way back in; `--load-set` learning to honour
/// a recorded capacity is what made the two disagree out loud.
///
/// It changes the bytes of Set files saved by older builds of this program.
/// Those files still load — a `capacity` record has always meant what it says —
/// and they go on describing whatever they described. What changes is that new
/// ones describe the run.
pub(crate) fn saving_capacities(args: &Args, l1s: &[karakuri_ir::typed::Checked]) -> Vec<u32> {
    l1s.iter().map(|l1| capacity_for(args, l1)).collect()
}

/// What a saved Set says each of its geometries is salted with: **what the run
/// it describes will be salted with**, one number per geometry.
///
/// The same function the run itself asks, for the reason [`saving_capacities`]
/// exists — a writer with its own copy of the rule records numbers the run was
/// not using, and the file then describes a picture nobody has seen. Slot 0's,
/// because a Set file describes one Set and slot 0 is the one that gets saved.
///
/// **Derived today and recorded from here on.** Nothing on the command line
/// assigns a salt, so these are the ordinals — and writing them down is exactly
/// what stops them being ordinals: `docs/ir-spec.md` says *where it came from
/// stops mattering once it is recorded*, and from this line onward the file is
/// where the value lives. Reordering the paths in `--set` moves the colours of
/// a Set that was never saved and no longer moves the colours of one that was.
pub(crate) fn saving_seeds(args: &Args, l1s: &[karakuri_ir::typed::Checked]) -> Vec<u32> {
    salts_for(seed_for(0), recorded_salts(args, 0), l1s.len())
}

/// Write the material as a Set file, and say where it went.
///
/// The first `--set` pair only. A Set file describes **one Set**, and a deck of
/// four is a session's arrangement rather than a Set's — that is the same line
/// `Record::is_set_state` draws, seen from the writing side.
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
    // **Every node, on the layer its own `kind` put it on**, and its source in
    // the store before the file that references it. The sorter already answered
    // the layer question for the engine — see [`sort_slot`] — so an L2, an L3
    // or a field is saved as what it is rather than refused for want of a slot
    // to write it in; the `put` is here rather than in the writer because this
    // is the caller holding paths that are still true. See `setfile::Node`.
    let nodes = match saving_nodes(&store, nodes) {
        Ok(nodes) => nodes,
        Err(e) => {
            eprintln!("karakuri-cli: {e}");
            std::process::exit(1);
        }
    };
    match setfile::save(
        &store,
        // `--save-set ID`, which is a flag an operator typed.
        Asked::Operator,
        id,
        setfile::Saving {
            nodes: &nodes,
            capacities: &saving_capacities(args, l1s),
            params: &args.overrides,
            bindings: &args.bindings,
            // **Which node fills each declared slot**, saved so the file
            // rebuilds: a slot nothing binds is refused where the Set is built,
            // so a Set file that dropped its edges would be one that no longer
            // loads.
            edges: &args.edges,
            camera: &camera,
            // **The flag's, because this path has no Set to ask.**
            // `--save-set` writes the material and exits before anything is
            // built, so what the run *would* play is what the flags say — and
            // for a one-shot they cannot be stale, since nothing has happened
            // to make them so. `k` and the MCP tool are the paths where that
            // stops being true, and `playing_values` reads the Set there.
            //
            // Through `layering_for`, so the flag and a `--load-set` file
            // cannot mean different things by it here than they mean anywhere
            // else — `--load-set` beside `--save-set` is refused, so the file
            // half is only ever the default today, and one derivation is still
            // one derivation.
            layering: layering_for(args, 0, recorded_layering(args, 0)),
            // **No selection, because nothing has selected.** `r` is a key
            // pressed at a running frame and there is no flag for it, so a
            // one-shot save has none to record — see `recorded_live`.
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

/// Read a Set file and fold what it says back into the arguments, so everything
/// downstream is driven the way the flags drive it.
///
/// **Every note is printed.** A Set file this build cannot honour in full still
/// loads, and the alternative — succeeding quietly — is the material being
/// subtly not what was saved with nothing anywhere saying so.
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
    // **`capacity_given` too, or the number is read and then discarded.** It
    // is what makes `capacity_for` stop falling back to the procedure's own
    // declared default — and a Set file that recorded a capacity is somebody
    // having said so as much as `--capacity` is. Without it, `--capacity 100000
    // --save-set x` followed by `--load-set x` ran at whatever the `.kir`
    // declared, which falsifies the one promise a Set file makes.
    // The first geometry's, because that is what one number can hold; the rest
    // travel per geometry on `from_set` and are applied where the sources are
    // in hand. This one is still needed as the flag: it is what a rebuilt slot
    // is given — see `Watch::new` — and what the status line prints.
    if let Some(capacity) = loaded.capacities.first().copied().flatten() {
        args.capacity = capacity;
        args.capacity_given = true;
    }
    // Appended rather than replacing: a `--param` or `--bind` given alongside
    // `--load-set` is the operator overriding the file, and the later value is
    // what `build` applies.
    let mut overrides = loaded.params.clone();
    overrides.append(&mut args.overrides);
    args.overrides = overrides;
    let mut bindings = loaded.bindings.clone();
    bindings.append(&mut args.bindings);
    args.bindings = bindings;
    // **The file's edges, then the flags'**, on the terms the params above
    // follow: an `--edge` given beside `--load-set` is the operator rebinding a
    // slot the file bound. It *replaces* rather than piling up, which is where
    // this differs from a `--param` — two edges on one slot are refused where
    // the Set is built, so appending both would turn an override into a
    // refusal. Only the slot the flag names is dropped; the file's other edges
    // stand.
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
        // **Carried rather than folded into `--merge`**, and read back through
        // [`layering_for`] — which is where the flag and the file meet, once,
        // for everything that builds this slot or writes it out again.
        layering: loaded.layering,
        live: loaded.live,
        capacities: loaded.capacities.clone(),
    });
    loaded
}

/// **Every save still in flight, collected until they are all in or `deadline`
/// passes.**
///
/// A free function over the channel rather than a loop inside
/// [`Live::awaited_saves`], so that the bound — which is the whole of what makes
/// waiting at the end of a run safe rather than a way to hang on a bad disk —
/// can be checked without a window and a GPU. That is the same reason
/// [`live_sources`] is a free function.
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
            // Timed out, or every sender is gone and nothing more can arrive.
            // Either way there is nothing left to wait for.
            Err(_) => break,
        }
    }
    landed
}

/// **One live save, from the frame that asked for it to the file on disk.**
pub(crate) struct Save {
    pub(crate) slot: usize,
    /// **Whose act this save is**, which decides the directory it lands in and
    /// is decided at the call site — see [`Live::save_set`] and
    /// [`karakuri_environment::Asked`].
    pub(crate) asked: Asked,
    pub(crate) id: String,
    /// The store root, not an open store: opening it creates directories, which
    /// is I/O, which belongs on the thread below rather than on a frame.
    pub(crate) root: PathBuf,
    pub(crate) sources: Sources,
    /// **What the file will say, with `nodes` still empty.** The nodes are the
    /// one part of a Set file that needs a store — a hash per source — so they
    /// are filled in where one is opened and never here.
    ///
    /// A half-built value crossing a thread boundary is worth a sentence,
    /// because the alternative was considered and is worse: a second struct
    /// holding "the other six fields" is a type whose whole content is which
    /// field it is missing, and it would have to be kept in step with
    /// `setfile::Owned` by hand forever.
    pub(crate) values: setfile::Owned,
}

impl Save {
    /// Write it. **Everything here is off the render thread**: opening a store
    /// creates directories, and the Set file itself is written and renamed into
    /// place.
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
        // **Before the file that references them**, which is what
        // [`setfile::Node`] carrying a hash asks of every caller: the writer
        // cannot check that a hash resolves without reading the store back, so
        // putting them is the caller's promise. See [`Sources::into_nodes`].
        values.nodes = sources.into_nodes(&store)?;
        setfile::save(&store, asked, &id, values.saving())
    }
}

/// What a live save came back with, at the frame it arrives.
pub(crate) struct Saved {
    pub(crate) slot: usize,
    /// Carried through so the sentence at the end names the right directory:
    /// the library's line tells an operator how to load it back, and the
    /// sandbox's cannot, because nothing loads one.
    pub(crate) asked: Asked,
    pub(crate) id: String,
    /// `Ok` and the file is on disk under `id`. **A failure is printed and no
    /// record is written**: a stream saying a save happened when the disk
    /// refused is exactly the shape of lie this codebase spends its comments
    /// refusing.
    pub(crate) outcome: Result<(), String>,
    /// Where a client that asked for this save is waiting, and `None` when a
    /// hand pressed `k`.
    ///
    /// **It rides the save rather than being looked up when the outcome lands.**
    /// A map from an id to whoever asked would be a second place that knows
    /// which save is which, and the outcome already carries everything needed to
    /// find its way home.
    pub(crate) reply: Option<mcp::Reply>,
}

/// A save that will not happen, to the terminal and to whoever asked if it was
/// not a hand.
///
/// **One sentence and one home.** Every refusal here reaches two audiences now,
/// and the way that goes wrong is a copy of the words for the second one — which
/// is free to be right on the day it is written and wrong at the next
/// correction. The wording of the refusal below has already needed one.
pub(crate) fn refused(reply: Option<mcp::Reply>, said: String) {
    eprintln!("{said}");
    if let Some(reply) = reply {
        reply.settled(Err(said));
    }
}
