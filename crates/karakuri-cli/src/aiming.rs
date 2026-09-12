use super::*;

/// **What every slot is running.**
///
/// One representation of "what bytes is this node running", held per slot as the
/// addresses a `procedure` record names — so a slot holding a chain and two
/// geometries has a line for each rather than an L1 and some renderers.
///
/// **It is seeded before the first frame**, which is the property [`Sources`]
/// leans on: every slot with files behind it has a hash from the outset. It was
/// previously seeded from nothing and filled in only by the watcher, so a slot
/// nothing had rebuilt had no hash anywhere and the live saver reached for the
/// *paths* instead. That gave the question two answers, and they disagree in
/// every state where something has rewritten a file the run is not drawing
/// from. A slot that is still `None` here is one with no files behind it at
/// all — see [`Running::at_launch`] — and it saves nothing rather than guessing.
///
/// **A type of its own rather than a field on `Live`**, because what a slot is
/// running is one fact with one transition: a build lands and it moves. It is
/// also what lets that transition be tested — the whole of it happens without a
/// window, a GPU or a governor.
///
/// **It held a second list until ADR-0316** — what a rollback would bring back
/// — because a rollback was the only thing that could name what it restored.
/// Nothing restores anything now: a version over the budget stays in the slot
/// with the slot stopped, and putting an earlier one back is a build like any
/// other, which lands here through [`Running::landed`] and is recorded like any
/// other.
///
/// **And the whole rule is in here**, which is a repair rather than a
/// restatement: the caller used to decide that a build it could not name was
/// not a swap at all, so it skipped the one half and took the other, and the
/// pair came apart in exactly the way this paragraph says it cannot. See
/// [`Running::landed`].
pub(crate) struct Running {
    pub(crate) playing: Vec<Option<Nodes>>,
}

impl Running {
    /// **Seed every slot from the material the run compiled**, addressed by the
    /// bytes that compile read.
    ///
    /// **At launch and for every windowed run**, not on `editable()`. A run with
    /// neither `--watch` nor `--mcp` is exactly the run where nothing will ever
    /// pick an edit up, so it is the run whose disk is most free to drift away
    /// from its picture — gating on `editable()` would leave that hole open.
    ///
    /// **No store, no disk, and nothing that can fail.** This used to open the
    /// store and write one artifact per node, which cost a plain windowed run a
    /// `.karakuri` directory it had never asked for — `docs/manual.md` says such
    /// a run "copies nothing and creates no directory", and it did until this
    /// function existed. The reason given for writing at launch was that a
    /// replay must resolve every hash a `procedure` record names; that is
    /// true, and it is true only of a run with a recorder. So the bytes go in where a recorder is opened
    /// (see `App::resumed`) and where a save actually happens (see
    /// [`Sources::into_nodes`]), and a run that does neither writes nothing.
    ///
    /// It also used to *re-read* each `.kir` here, seconds after the compile
    /// that produced the deck. [`Placed::source`] is why it no longer can.
    pub(crate) fn at_launch(placed: &[Vec<Placed>], slots: usize) -> Running {
        let mut playing: Vec<Option<Nodes>> = vec![None; slots];
        for (slot, nodes) in placed.iter().enumerate().take(slots) {
            if nodes.is_empty() {
                continue;
            }
            playing[slot] = Some(stored_nodes(nodes.iter().map(Placed::node).collect()));
        }
        Running { playing }
    }

    /// What `slot` is running, or `None` for a slot whose sources are not in the
    /// store.
    pub(crate) fn playing(&self, slot: usize) -> Option<&Nodes> {
        self.playing.get(slot).and_then(Option::as_ref)
    }

    /// **A build landed.** What the slot is now running, for the stream to say.
    ///
    /// `nodes` is `None` when that build's sources never reached the store —
    /// the watcher says so at the time, and the addresses it would have named
    /// do not exist. **That is still a swap**, and taking it as one is the
    /// whole of what this argument is for: the slot is on something new, and it
    /// has no address until the next build lands. `None` comes back and no
    /// `procedure` record is written, because there is nothing to name.
    ///
    /// **It is still a swap when the budget's verdict goes against it**, too: a
    /// version that costs more than one frame may is in the slot with the slot
    /// stopped (ADR-0316), so it is what a save of that slot writes down and
    /// what a record names. What is not true of it is that the slot is running,
    /// which the status line says and this list does not.
    ///
    /// **The decision used to live in the caller**, which returned early on a
    /// build it could not name and so applied half a transition rule: the swap
    /// was skipped here and the matching rollback was not, and after one such
    /// pair the slot was recorded as running the version *before* the one on
    /// screen. `k` then wrote that version down and a recorded run put
    /// `procedure` records naming it into the stream — the picture and the file
    /// disagreeing, silently, which is the failure this whole type exists to
    /// make impossible. There is no rollback left for that pair to come apart
    /// across, and this is still where the transition happens.
    pub(crate) fn landed(&mut self, slot: usize, nodes: Option<Nodes>) -> Option<Nodes> {
        self.playing[slot] = nodes;
        self.playing[slot].clone()
    }
}

/// [`setfile::Node`]s as the addresses [`Nodes`] holds — the layer spelled the
/// way a record spells it, so a slot seeded at launch and a slot the watcher
/// rebuilt are the same shape.
pub(crate) fn stored_nodes(nodes: Vec<setfile::Node>) -> Nodes {
    nodes
        .into_iter()
        .map(|node| (setfile::kind_name(node.layer), node.index, node.hash))
        .collect()
}

/// **What a Set file says about the Set that is playing**, read off that Set.
///
/// Everything except the nodes, which are the one part a store has to be
/// involved in — see [`Save`].
///
/// **Seven of the eight are read from the Set and not from `Args`**, and the
/// eighth is the exception that has to earn itself — which is the decision this
/// function exists to hold. The reason is [`saving_capacities`]'s,
/// stated once and true of all of them: a writer with its own copy of the rule
/// records numbers the run was not using, and the file then describes a picture
/// nobody has seen. A run that has been *played* makes that concrete rather than
/// theoretical — a param moves through a record, and a slot rebuilt from an
/// edited `.kir` can change its own declared capacity underneath the flag that
/// was never given.
///
/// - **capacities** come per geometry from [`Set::source_capacities`] and not
///   from `Set::capacity`, which is the sum. One number for a two-geometry Set
///   is neither geometry's, and the writer refuses it.
/// - **params** are written *addressed*, every declaration of every node,
///   where `--save-set` writes only the `--param`s it was given. That is more
///   lines and it is the right ones: the Set holds a value per node whether an
///   operator wrote it or a `.kir` declared it, and a file that recorded only
///   the overrides would come back different the day the declaration changed.
/// - **bindings** come back with the ranges and curves they are riding at.
/// - **edges** are the run's — see `Live::edges` for why this one is not the
///   Set's, and why that is a copy of a value rather than of a rule.
/// - **camera** is the built-in orbit's six numbers, which a `camera` record
///   and a Set file both set from outside.
/// - **layering** is [`Set::layering`] and emphatically *not* `--merge`. This
///   is the surface `k` and the MCP tool reach, and both exist to write **what
///   is on screen**: the slot may have been filled by `--load-set` from a file
///   that recorded a `merge` the flags never mentioned, and it may have been
///   hot-swapped since. Asking the flag would write a file describing a Set
///   nobody was watching, which is [`saving_capacities`]' failure exactly.
/// - **live** is read off [`Set::inputs`] by [`selected_renderer`], for the
///   same reason and a louder one: nothing but the run can know it. There is no
///   flag that selects a renderer — `r` does, mid-performance — so the Set is
///   not merely the better source here, it is the only one.
/// - **seeds** are [`Set::source_salts`], one per geometry: what it *is* salted
///   with rather than what a position in `--set` would derive.
pub(crate) fn playing_values(
    set: &karakuri_engine::Set,
    edges: &[karakuri_engine::set::Edge],
) -> setfile::Owned {
    setfile::Owned {
        // Filled where a store is open, and nowhere else.
        nodes: Vec::new(),
        capacities: set.source_capacities(),
        params: set
            .params()
            .map(|(layer, index, key, value)| {
                karakuri_engine::ParamWrite::at(layer, index, key, value)
            })
            .collect(),
        bindings: set.bindings().to_vec(),
        edges: edges.to_vec(),
        // **`Set::orbit` and not the `Set::camera` field**: three of the six
        // are the camera node's parameters, so the field is what was last
        // stated and the map is what a hand, a binding or a carried ride left
        // there (ADR-0318).
        camera: set.orbit(),
        layering: set.layering(),
        live: selected_renderer(set.inputs()),
        seeds: set.source_salts().to_vec(),
    }
}

/// **Which renderer a Set is folded to**, as a `merge` record spells it:
/// `Some(i)` where exactly one input is live, and `None` where every one of
/// them is.
///
/// **Every-live is checked first, and that decides the one-renderer case.** A
/// composited Set holding a single renderer has one live input, which is both
/// "all of them" and "exactly one" — and it is the first, because such a Set is
/// one nobody has selected in. Writing `live 0` for it would record a choice
/// that was never made, and `Record::Merge` is explicit that absent means every
/// input live rather than node 0.
///
/// **Anything else is `None` too, and the anything else has no producer.**
/// `mix::select` is the only thing that clears a `live` flag and it always
/// leaves exactly one set, so a fold with two of five live cannot be reached
/// from any surface this program has. If one ever is, `None` records the Set as
/// unselected — which is a fold the reader can build — rather than naming one
/// of them and calling that the choice.
pub(crate) fn selected_renderer(inputs: &[karakuri_engine::mix::Input]) -> Option<u32> {
    if inputs.iter().all(|input| input.live) {
        return None;
    }
    let mut live = inputs.iter().enumerate().filter(|(_, input)| input.live);
    match (live.next(), live.next()) {
        (Some((at, _)), None) => Some(at as u32),
        _ => None,
    }
}

/// **One slot's watcher, and the aim it is pointed at.**
///
/// **The run holds this so that a rewiring can reach the build worker.** The
/// wiring a slot rebuilds with is not on disk anywhere — `Args::edges` at
/// launch, `watch::Watch::edges` on every rebuild, [`Live::edges`] at a save —
/// so an edge written during a show has to be *handed* to the watcher, and
/// `watch::Watch::aimed_by` is the one way in. It is deliberately not an
/// install: `Deck::install` would put a Set on air that nothing measured, and
/// an aim instead says *look at this instead* and lets go, after which
/// everything is the path an edit already takes — compiled on the worker,
/// swapped at a frame boundary, judged against the budget, and left in the
/// slot with the slot stopped if it costs too much. That is [`mcp::WireRequest`]'s second point,
/// and reaching it this way is why there is no second route into a slot.
///
/// **The aim is kept and not only the sender**, because an `Aim` is every field
/// of the slot's identity and *anything left out comes back as the outgoing
/// slot's* — a fold silently un-selected, a camera back at `Orbit::default()`,
/// salts that repaint every element. A rewiring changes one field of thirteen,
/// so the other twelve have to be restated from somewhere, and this is that
/// somewhere: what the watcher was started at, moved forward by every aim sent
/// since.
pub(crate) struct Aiming {
    /// The other end of `watch::Watch::aimed_by`'s channel, for this slot's
    /// watcher and no other. A watcher re-pointed through somebody else's
    /// sender would rebuild a deck nobody named.
    pub(crate) aim: std::sync::mpsc::Sender<watch::Aim>,
    /// **Where that watcher is pointed**, kept in step with what has been sent:
    /// the values it was constructed with until the first aim, and the last aim
    /// after that. A copy that stopped being updated would restate a stale
    /// wiring on the *second* rewiring of a run, which is the hardest version
    /// of this mistake to see.
    pub(crate) at: watch::Aim,
}

impl Aiming {
    /// **Point the watcher at the same material with `edges` instead**, and
    /// answer whether it is still there to be pointed.
    ///
    /// `Err` is a build worker that has ended — the receiver is gone — which is
    /// a run shutting down. It is reported rather than swallowed: the edge is
    /// in the run's wiring either way, and *nothing will rebuild* is a
    /// different fact from *the slot is recompiling*.
    pub(crate) fn re_aim(&mut self, edges: Vec<karakuri_engine::set::Edge>) -> Result<(), ()> {
        self.at.edges = edges;
        self.aim.send(restated(&self.at)).map_err(|_| ())
    }
}

/// One aim, said again — because `watch::Aim` is not `Clone` and a re-point
/// restates every field of it.
///
/// **No `..` on either side of this**, which is `Watch::repointed`'s own rule
/// met from the sending end: it destructures with no `..` so that a field
/// added to `Aim` cannot be left behind, and a *sender* that filled the new
/// field with a default would defeat that from here. The compiler names every
/// one of them, so the day another arrives this stops compiling rather than
/// quietly re-aiming a slot at it.
pub(crate) fn restated(aim: &watch::Aim) -> watch::Aim {
    let watch::Aim {
        head,
        rest,
        layering,
        live,
        capacity,
        seed_salt,
        salts,
        camera,
        overrides,
        published,
        bindings,
        edges,
        authorities,
        set,
    } = aim;
    watch::Aim {
        head: head.clone(),
        rest: rest.clone(),
        layering: *layering,
        live: *live,
        capacity: *capacity,
        seed_salt: *seed_salt,
        salts: salts.clone(),
        camera: *camera,
        overrides: overrides.clone(),
        published: published.clone(),
        bindings: bindings.clone(),
        edges: edges.clone(),
        authorities: authorities.clone(),
        // **The Set the slot is running, said again like everything else.** A
        // re-aim changes the wiring and nothing about what is playing, so a
        // rewiring that dropped this would move every version written after it
        // into a chain under no Set at all.
        set: set.clone(),
    }
}

/// **Every edge a client asked for on one frame, applied to the run's wiring
/// and answered.**
///
/// This is [`mcp::WireRequest`]'s three points, and it is a free function so
/// that all three are checkable without a window, a GPU or a `Deck` — the
/// wiring, the re-aim and the sentence are the whole of what this decides, and
/// none of them needs one.
///
/// # Replace, keyed on the input
///
/// An edge is dropped and the new one appended, keyed on `(node, slot)` — the
/// node that declares the input and what its procedure calls it — which is
/// `--edge`'s own spelling beside `--load-set` a few hundred lines up, down to
/// the `retain` and the order it leaves behind. It is forced rather than
/// chosen: `SetError::SlotBoundTwice` refuses two edges on one input where the
/// Set is built, so an append would make the *second* call on an input a
/// refusal and leave a model unable to change its mind.
///
/// **The key does not include the deck slot, because the run's wiring does
/// not.** `Live::edges` is one list for the whole run and an edge naming a node
/// a Set has not got is passed over where the Set is built — see
/// `karakuri_engine::set::Wiring::edges`. So a request names a deck slot to say
/// *which slot rebuilds*, and two slots holding a node of the same name share
/// one entry in this list, exactly as they do when `--edge` is typed on the
/// command line.
///
/// # A slot this deck does not hold
///
/// **Refused, in [`no_such_slot`]'s words, and nothing is rewired** — the
/// decision [`Live::save_set`] already makes for a save and for its reason: a
/// key press cannot name a slot the deck has not got and a tool call can, and
/// this is the guard that does not depend on the surface that asked having one.
/// The MCP server checks the number against `mcp::Slots` before it sends, so a
/// model meets its refusal there; the deck's own count is a thing only the run
/// knows, and this is where it is known.
///
/// **Residency is deliberately not consulted.** An off-air slot is wired and
/// rebuilt like any other: a slot is prepared while it is dark and put on air
/// afterwards, so refusing an edge on an allocated slot would forbid the one
/// order an operator actually works in. What a rebuild of a dark slot costs is
/// the same as any other rebuild and is judged the same way.
///
/// # The same input wired twice on one frame
///
/// **Every request is applied, in the order it arrived, and the last one is
/// what the run is wired with** — a rewiring is a model changing its mind, and
/// the frame a change of mind lands on is not something a client controls. One
/// aim per slot goes out after all of them are in the list, so the rebuild
/// carries the settled wiring rather than an intermediate one, and
/// `Watch::repointed` takes only the newest aim anyway.
///
/// **A request the same frame overwrote is told so**, which is the only part of
/// this that costs anything: its edge *was* written and then replaced, and a
/// reply saying only "wired" would be a true sentence about a state the run no
/// longer holds by the end of the frame it was sent on.
///
/// # What each answer is
///
/// `Err` is the refusal above and nothing else. A slot with no watcher is not a
/// failure — the edge is in the run's wiring, a `save_set` records it, and the
/// only thing missing is the rebuild, which the sentence says. That matches the
/// note `mcp::wire_input` already adds for a run started without `--watch`.
pub(crate) fn rewired(
    asked: &[(usize, karakuri_engine::set::Edge)],
    edges: &mut Vec<karakuri_engine::set::Edge>,
    aims: &mut [Option<Aiming>],
    slot_count: usize,
) -> Vec<Result<String, String>> {
    // **Every edge into the list before any watcher is re-aimed**, so that a
    // frame carrying two of them rebuilds once, at the wiring the frame ended
    // with.
    let mut said: Vec<Option<Result<String, String>>> = asked.iter().map(|_| None).collect();
    let mut named: Vec<usize> = Vec::new();
    for (at, (slot, edge)) in asked.iter().enumerate() {
        if !slot_in_range(*slot, slot_count) {
            said[at] = Some(Err(format!(
                "{}, and nothing was rewired",
                no_such_slot(*slot, slot_count)
            )));
            continue;
        }
        edges.retain(|held| !(held.node == edge.node && held.slot == edge.slot));
        edges.push(edge.clone());
        if !named.contains(slot) {
            named.push(*slot);
        }
    }
    // `None` for a slot with no watcher at all, `Some(false)` for one whose
    // build worker has ended: two different things to say, and neither of them
    // is "the slot is recompiling".
    let rebuilding: Vec<(usize, Option<bool>)> = named
        .into_iter()
        .map(|slot| {
            let state = aims
                .get_mut(slot)
                .and_then(Option::as_mut)
                .map(|aiming| aiming.re_aim(edges.clone()).is_ok());
            (slot, state)
        })
        .collect();
    for (at, (slot, edge)) in asked.iter().enumerate() {
        if said[at].is_some() {
            continue;
        }
        let mut line = format!(
            "slot {slot}: wired `{}.{}={}`",
            edge.node, edge.slot, edge.to
        );
        // Keyed on the input alone, like the replacement above: whichever deck
        // slot a later request named, it took this entry in the run's wiring.
        //
        // **Only ones that were applied**, which is the whole reason this is a
        // second pass rather than a lookahead in the first: a later request
        // refused for its slot number wrote nothing, and telling this one it
        // had been replaced by an edge that never landed would be the same lie
        // in the other direction.
        let over = asked[at + 1..].iter().find(|(later_slot, later)| {
            slot_in_range(*later_slot, slot_count)
                && later.node == edge.node
                && later.slot == edge.slot
        });
        if let Some((_, later)) = over {
            let _ = write!(
                line,
                ", and a later request on this frame replaced it with `{}` — the run is \
                 wired with that one and it is what the rebuild carries",
                later.to
            );
        }
        let state = rebuilding
            .iter()
            .find(|(named, _)| named == slot)
            .and_then(|(_, state)| *state);
        let tail = match state {
            Some(true) => {
                " — the slot is recompiling with it, and `swap_outcome` says what the build \
                 made of it"
            }
            Some(false) => {
                " — this slot's build worker has ended, so nothing will rebuild: the edge is \
                 the run's from here on and a `save_set` of this slot records it"
            }
            None => {
                " — this slot has no watcher, so nothing rebuilds: what is on air was built \
                 with the wiring the run started with, and a `save_set` of this slot records \
                 the edge"
            }
        };
        line.push_str(tail);
        said[at] = Some(Ok(line));
    }
    // Every entry was filled by one of the two loops above: the first answers
    // the refusals and the second answers everything it skipped.
    said.into_iter().map(Option::unwrap).collect()
}

/// `meters` is false for the offscreen paths: a `--render` has nobody to show
/// a level to, and a meter that nothing reads is a compute pass and a staging
/// ring per frame for no reason. That is the whole point of it being opt-in.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_deck(
    gpu: &Gpu,
    procs: &[Material],
    args: &Args,
    watch: bool,
    meters: bool,
    width: u32,
    height: u32,
    // Where a rebuilt procedure's sources are put and reported. `None` and no
    // watcher touches a store. See `watch::Watch::stored` for why this is no
    // longer the recorder's switch.
    stored: Option<(
        std::sync::Arc<karakuri_store::store::Store>,
        std::sync::mpsc::Sender<watch::Built>,
    )>,
    // The run's edit history, shared by every slot's watcher and already
    // holding what the run started with. `None` for a run that cannot be
    // edited — see `history`.
    snapshots: Option<history::Shared>,
    // **The deck, and where each of its slots can be re-pointed** — one entry
    // per slot, `None` for every slot with no watcher behind it. The senders
    // are made where the watchers are, because a sender paired with the wrong
    // slot's watcher would rebuild a deck the caller did not name; see
    // [`Aiming`].
) -> (Deck, Vec<Option<Aiming>>) {
    // One flag per binding, shared across every slot: a binding names a layer
    // and a param, and a deck of four slots is four chances for it to land.
    let mut attached = vec![false; args.bindings.len()];
    // **Resolved once per slot, and handed to everything that needs it.** A
    // slot's salts are what its Set is built with *and* what its watcher
    // restates on every rebuild; working them out in two places is how a save
    // under `--watch` would come back in different colours from the Set it
    // rebuilt.
    let salts: Vec<Vec<u32>> = procs
        .iter()
        .enumerate()
        .map(|(slot, material)| {
            salts_for(
                seed_for(slot),
                recorded_salts(args, slot),
                material.l1s.len(),
            )
        })
        .collect();
    let built: Vec<(HotSwap, Option<Aiming>)> = procs
        .iter()
        .enumerate()
        .map(|(slot, material)| {
            let (l1, l2s, l3s, fields, l4s) = (
                material.l1s.as_slice(),
                &material.l2s,
                material.l3s.as_slice(),
                material.fields.as_slice(),
                &material.l4s,
            );
            // **Read once for both the Set and its watcher** — see
            // [`recorded_camera`].
            let camera = recorded_camera(args, slot);
            // **The flag and the file, resolved once** — see [`layering_for`].
            // Read here rather than at each use for [`recorded_camera`]'s
            // reason and with the same symptom: the watcher restates a layering
            // on every rebuild, so a second derivation that disagreed would
            // turn the first save of any `.kir` in the slot into a Set that
            // stopped compositing, with nothing said.
            let layering = layering_for(args, slot, recorded_layering(args, slot));
            let live = recorded_live(args, slot);
            let set = build(
                gpu,
                l1,
                l2s,
                l3s,
                fields,
                l4s,
                layering,
                &material.names,
                &args.edges,
                &capacities_for(args, l1, recorded_capacities(args, slot)),
                &mut attached,
                &args.overrides,
                &args.bindings,
                &args.published,
                // **The Set's own seed is its first geometry's salt**, which
                // is what one number can hold and what a file recording one
                // `seed` has always meant by it. A Set file's own where it
                // recorded one, so a saved Set reproduces rather than being
                // re-salted by the slot it lands in — which only ever applies
                // to slot 0, since `--load-set` fills that slot and the rest
                // come from `--set`.
                salts[slot]
                    .first()
                    .copied()
                    .unwrap_or_else(|| seed_for(slot)),
                &salts[slot],
                camera,
                live,
            );
            if watch {
                // **What this slot's watcher is pointed at, stated once.** It
                // used to be thirteen arguments to `Watch::new` and it is now
                // an `Aim` those arguments are read out of, because a re-point
                // has to restate every one of them — see [`Aiming`] and
                // `watch::Aim`, whose own documentation is that *anything left
                // out comes back as the outgoing slot's*. Building the aim here
                // and starting the watcher from it is what makes "where this
                // watcher is pointed" one value rather than two lists that
                // agree today.
                let at = watch::Aim {
                    // **With the names, not only the paths.** A rebuild
                    // resolves its edges against them, and a watcher that
                    // handed the sort bare paths would rename every node
                    // on the first save.
                    head: args.sets[slot].0.clone(),
                    rest: args.sets[slot].1.clone(),
                    // **The same reading the Set was built with**, and
                    // the whole reason it is read once above: a rebuild
                    // restates the layering rather than re-deriving it, so
                    // a slot that loaded a composited Set file is still
                    // compositing after the first save of a `.kir` in it.
                    // A rebuild that let this be worked out again from
                    // `--merge` alone would quietly discard what was
                    // loaded — which is `Watch::camera`'s failure, in the
                    // one place the picture does not even come back.
                    layering,
                    // **And the selection with it**, for the same reason
                    // and in the same breath: a fold restated to every
                    // input live is a rebuild silently un-selecting what
                    // the file selected. See `Watch::live`.
                    live,
                    capacity: args.capacity_given.then_some(args.capacity),
                    seed_salt: salts[slot]
                        .first()
                        .copied()
                        .unwrap_or_else(|| seed_for(slot)),
                    // **Restated on every rebuild rather than derived
                    // there.** A slot filled from a Set file is running at
                    // the salts that file recorded, and a rebuild that
                    // derived its own would change every colour in it on the
                    // next save of a `.kir`.
                    salts: salts[slot].clone(),
                    // **The same reading the Set was built with**, and
                    // stated as an `Orbit` rather than as the `Option` it
                    // was read as: a slot that loaded no `camera` record is
                    // running at `Orbit::default()`, because that is what
                    // `Set::build_many` builds and what leaving it
                    // unassigned above therefore means. See `Watch::camera`
                    // for why the option buys nothing past this line.
                    camera: camera.unwrap_or_default(),
                    overrides: args.overrides.clone(),
                    published: args.published.clone(),
                    bindings: args.bindings.clone(),
                    // **The run's wiring, which is one list and not one per
                    // slot** — see `Live::edges`. `wire_input` replaces an
                    // entry in it and re-aims this watcher with the result.
                    edges: args.edges.clone(),
                    authorities: Vec::new(),
                    // **The Set this slot is about to run**, which is the same
                    // answer the launch-time seed was given and for its
                    // reason: `--load-set` fills slot 0 and is refused
                    // alongside `--set`, so it is the only slot that can have
                    // one and every other is running a pair somebody typed.
                    // Nothing on this surface loads a Set into a running slot,
                    // so this is where the id is stated and `restated` is the
                    // only thing that moves it.
                    set: match slot {
                        0 => args.load_set.clone(),
                        _ => None,
                    },
                };
                // **Opened for every watched slot rather than only under
                // `--mcp`**, because the sender is what pairs a slot with its
                // own watcher and pairing it later would mean holding the
                // values above somewhere else to do it. A run nobody
                // rewires never sends on it and it costs a `Sender`.
                let (aim, aimed) = std::sync::mpsc::channel();
                // One worker and one watcher per slot, over that slot's own
                // two files. That is what makes "the slot whose files changed"
                // the thing that rebuilds: no slot can see another's edit.
                let swap = HotSwap::new(&gpu.device, &gpu.queue, set, args.budget_ms, {
                    let watcher = watch::Watch::new(
                        slot,
                        at.head.clone(),
                        at.rest.clone(),
                        at.layering,
                        at.live,
                        at.capacity,
                        at.seed_salt,
                        at.salts.clone(),
                        at.camera,
                        at.overrides.clone(),
                        at.published.clone(),
                        at.bindings.clone(),
                        at.edges.clone(),
                        at.authorities.clone(),
                    )
                    // **Where a re-point arrives.** Every slot this program
                    // watches can be re-aimed from the render thread, which is
                    // how an edge written over MCP reaches the build worker:
                    // `Watch::aimed_by`'s own documentation says a load *"lets
                    // go"* and everything after it is the path an edit already
                    // takes, which is exactly what `WireRequest` asks for.
                    .aimed_by(aimed);
                    // The history is kept whether or not a session is
                    // being recorded: the two answer different questions —
                    // see `Watch::snapshots`. One `Shared` across every
                    // slot, because it is also what the launch-time seed
                    // wrote into.
                    let watcher = match &snapshots {
                        // **The same answer the seed was given**, so a slot's
                        // starting version and everything it is edited into are
                        // filed under one Set rather than under two. Slot 0 is
                        // the only one that can have an id, because
                        // `--load-set` fills it and is refused alongside
                        // `--set`.
                        //
                        // **Read off the aim rather than worked out again
                        // here.** The aim is what a re-point restates and what
                        // moves this id from now on, so a second `match slot`
                        // at this line would be a second answer to *what is
                        // this slot running* the day anything on this surface
                        // loads a Set into a running slot
                        // (`docs/principles/0087-name-the-property-never-the-shape.md`).
                        Some(shared) => watcher.snapshotting_to(shared.clone(), at.set.clone()),
                        None => watcher,
                    };
                    // **Whenever the run is editable**, which is where the
                    // caller decides it — the watcher only has to be told
                    // where. This used to be "only when a session is being
                    // recorded", and see `Watch::stored` for what that cost.
                    Box::new(match &stored {
                        Some((store, tx)) => watcher.storing_to(store.clone(), tx.clone()),
                        None => watcher,
                    })
                });
                (swap, Some(Aiming { aim, at }))
            } else {
                // **No watcher and therefore nothing to re-aim.** An edge
                // written into a run like this one is still the run's — a save
                // records it — and nothing rebuilds, which is what
                // [`rewired`] says in that slot's sentence.
                (HotSwap::fixed(set), None)
            }
        })
        .collect();
    let (swaps, aims): (Vec<HotSwap>, Vec<Option<Aiming>>) = built.into_iter().unzip();
    let mut deck = Deck::new(&gpu.device, swaps, width, height);
    // The session's one local oscillator, before the first frame. `SEED` is
    // the seed every noise stream comes off — the same explicit seed the Sets
    // are salted from, so a run is reproducible from its arguments alone.
    deck.set_signals(Signals::new(args.bpm, u64::from(SEED)));
    if meters {
        deck.enable_meters(&gpu.device);
    }
    // Once, not per slot: what attached, and how far each will actually move.
    // The confidence is the part worth printing — a binding to an invented
    // signal moving a tenth of the way is the system working, and an operator
    // who does not know that reads it as a broken binding.
    // **Only the ones that attached.** Describing a binding that landed
    // nowhere told an operator it "decides that param outright" one line after
    // saying it was ignored, which is the failure the confidence display had
    // and had fixed — reappearing one step further along.
    for (binding, _) in args.bindings.iter().zip(&attached).filter(|(_, on)| **on) {
        eprintln!("  {}", describe(binding, deck.signals()));
    }
    (deck, aims)
}

/// One binding, in a line, ending with what it will do rather than only what
/// it says.
fn describe(binding: &Binding, signals: &Signals) -> String {
    // **A published control is not on the bus**, and asking the bus about it
    // gets the answer for a name nothing measures — zero, which reads as a
    // binding that will do nothing. It is the operator's hand: confidence 1,
    // and the Set resolves it. See `karakuri_engine::binding::CONTROL_PREFIX`.
    let confidence = if binding.signal.starts_with(CONTROL_PREFIX) {
        1.0
    } else if binding.signal == NOISE_SIGNAL {
        signals.noise(&binding.noise.unwrap_or_default()).confidence
    } else {
        signals.sample(&binding.signal).confidence
    };
    let effect = if binding.signal.starts_with(CONTROL_PREFIX) {
        "the published control decides it outright".to_string()
    } else if confidence >= 1.0 {
        "the signal decides it outright".to_string()
    } else {
        format!(
            "it moves {:.0}% of the way and the param's own value holds the rest",
            confidence * 100.0
        )
    };
    format!(
        "bind {:?} {} <- {} through {} onto [{}, {}] — confidence {confidence:.2}, so {effect}",
        binding.layer,
        binding.key,
        binding.signal,
        binding.curve.name(),
        binding.range[0],
        binding.range[1],
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build(
    gpu: &Gpu,
    l1s: &[karakuri_ir::typed::Checked],
    l2s: &[karakuri_ir::typed::Checked],
    // The cameras, in node order — see `Material::l3s`. Empty leaves the Set
    // looking from the built-in orbit.
    l3s: &[karakuri_ir::typed::Checked],
    // The fields, in node order — see `Material::fields`. Empty for a Set that
    // evaluates none.
    fields: &[karakuri_ir::typed::Checked],
    l4s: &[karakuri_ir::typed::Checked],
    layering: karakuri_engine::set::Layering,
    // What each node is called — see `Names`.
    names: &Names,
    // Which node fills each declared input slot — see `Args::edges`. Every one
    // the run was given, including any about another slot's Set, which the
    // engine passes over.
    edges: &[karakuri_engine::set::Edge],
    // One per entry in `l1s`, in the same order — see `capacities_for`.
    capacities: &[u32],
    // Set to `true` for each binding that attached, and left alone otherwise.
    // One flag per binding, and a caller building several slots ORs them: a
    // binding is worth describing if it attached *anywhere*.
    attached: &mut [bool],
    overrides: &[ParamWrite],
    bindings: &[Binding],
    published: &[karakuri_engine::set::Published],
    seed: u32,
    // One per entry in `l1s`, in the same order — see `salts_for`.
    salts: &[u32],
    camera: Option<karakuri_engine::camera::Orbit>,
    // Which renderer the Set comes up folded to — see `recorded_live`. `None`
    // leaves every input live, which is what `Set::build_many` builds and what
    // a Set nobody has selected in is.
    live: Option<u32>,
) -> Set {
    let deform: Vec<&karakuri_ir::typed::Checked> = l2s.iter().collect();
    let look: Vec<&karakuri_ir::typed::Checked> = l3s.iter().collect();
    let shapes: Vec<&karakuri_ir::typed::Checked> = fields.iter().collect();
    let draw: Vec<&karakuri_ir::typed::Checked> = l4s.iter().collect();
    // **Each source at the capacity it declares**, and `--capacity` overrides
    // all of them — one number cannot serve two L1s with different ranges.
    //
    // Asserted rather than zipped and hoped for: `zip` on a short list drops a
    // whole geometry, and a Set silently missing its second source is the same
    // picture as a Set that was never given one.
    assert_eq!(
        l1s.len(),
        capacities.len(),
        "one capacity per geometry source"
    );
    let sources: Vec<(&karakuri_ir::typed::Checked, u32)> =
        l1s.iter().zip(capacities.iter().copied()).collect();
    // **Resolved by the caller, never left to be filled in here.** The engine
    // derives a salt for a source nobody assigned one, and a run whose salts
    // were half assigned and half derived would be a run whose Set file records
    // numbers it was not using — so `salts_for` answers for every geometry and
    // this hands the whole answer over.
    assert_eq!(l1s.len(), salts.len(), "one salt per geometry source");
    let assigned: Vec<Option<u32>> = salts.iter().copied().map(Some).collect();
    match Set::build_many(
        &gpu.device,
        &gpu.queue,
        &sources,
        &deform,
        &look,
        &shapes,
        &draw,
        layering,
        seed,
        &assigned,
        karakuri_engine::set::Wiring {
            l1s: &names.l1s,
            l2s: &names.l2s,
            l3s: &names.l3s,
            l4s: &names.l4s,
            fields: &names.fields,
            edges,
        },
    ) {
        Ok(mut set) => {
            // **What the slot's nodes ended up called**, read off the Set
            // rather than worked out here: a name nobody wrote is derived in
            // `build_many`, and deriving it a second time to print it is the
            // shape this repository keeps finding wrong.
            eprintln!("  nodes: {}", set.node_names().join(", "));
            // **The `camera` record, into the node it is about.** It used to
            // be applied only to a Set whose files declared no L3, because an
            // L3 wrote the one camera state there was and an orbit assigned
            // beside it would have been overwritten before the first draw. The
            // orbit is its own node now — the last one, whatever else the Set
            // holds — so the record reaches it either way, and whether any
            // renderer draws from it is what an `edge` says rather than what
            // the file list happens to contain.
            if let Some(camera) = camera {
                // **Through `aim_camera`, which states the three placement
                // numbers into the camera node's parameter map as well.**
                // Assigning the field alone would leave the map holding the
                // numbers the Set was built with, and the picture reads the map
                // (ADR-0318).
                set.aim_camera(camera);
            }
            // **The `merge` record's selection, where the `camera` record's
            // six numbers go** — before the params and for their reason: a
            // value the file recorded is applied to the Set the file built,
            // once, in the one place that knows both. A Set comes up with
            // every input live, so leaving this out is a composited Set
            // loading back unselected, which is the half of a variant pool
            // that made saving one pointless.
            //
            // **Ineffective under `Overdraw`, and said rather than refused**,
            // which is `Set::select_renderer`'s own rule: a file that records
            // no `merge` records no selection either, so the only way to reach
            // this with an overdrawing Set is `--merge` left off a file that
            // has one — and `layering_for` decides that one line above.
            if let Some(at) = live {
                if !set.select_renderer(at as usize) {
                    eprintln!(
                        "  this set selects renderer {at} and has {} — every renderer is \
                         live",
                        l4s.len()
                    );
                }
            }
            for write in overrides {
                // **The refusal is the engine's sentence, printed rather than
                // reworded** — a `--param` and a `param` record reach the same
                // wall in the same words, which is
                // `docs/principles/0090-a-surface-offers-it-never-decides.md`.
                // Unreachable in a run today, because nothing grants a node's
                // authority yet and a Set nobody has spoken for lands
                // uniformly; the day a grant arrives, this line is what an
                // operator reads.
                match set.write_param(write) {
                    Ok(0) => eprintln!("  no parameter named `{}`, ignoring", write.key),
                    Ok(_) => {}
                    Err(refused) => eprintln!("  {refused}"),
                }
            }
            // After the overrides: a binding blends from the param's value, so
            // a `--param` on a bound param is the base of the blend rather
            // than a competitor for the write.
            // **Before the bindings**, because a macro is a binding whose source
            // is a published control: attaching one before the control existed
            // would be attaching it to a name nothing answers, and it would hold
            // its param where it found it for the rest of the run.
            for control in published {
                let name = control.name.clone();
                if let Err(e) = set.publish(control.clone()) {
                    eprintln!("  `{name}` is not published: {e}");
                }
            }
            for (at, binding) in bindings.iter().enumerate() {
                let (layer, key) = (binding.layer, binding.key.clone());
                match set.bind(binding.clone()) {
                    karakuri_engine::set::Bound::Yes => attached[at] = true,
                    karakuri_engine::set::Bound::NoSuchParam => {
                        eprintln!("  no {layer:?} parameter named `{key}` to bind, ignoring");
                    }
                    // **The other half of the same sentence.** This used to
                    // print the line above, which sends whoever reads it to
                    // look at the param — and the param is fine.
                    karakuri_engine::set::Bound::NoSuchControl => eprintln!(
                        "  `{}` is not published by this Set, so {layer:?} `{key}` is \
                         not bound",
                        binding.signal
                    ),
                }
            }
            set
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
