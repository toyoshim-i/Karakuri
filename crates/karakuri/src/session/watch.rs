use crate::{slot_in_range, Aiming, Sources};
use karakuri_engine::set::Layering;
use karakuri_engine::{Gpu, HotSwap, Set, DEFAULT_BUDGET_MS};
use karakuri_environment::{history, watch};
use karakuri_store::store::Store;

/// Every edge a client asked for on one frame, applied to the run's wiring and
/// answered.
///
/// This is `karakuri_mcp::WireRequest`'s three points, and it is a free
/// function so that all three are checkable without a window, a GPU or a `Deck`
/// — the wiring, the re-aim and the sentence are the whole of what this
/// decides, and none of them needs one. `karakuri-cli`'s `rewired` is the same
/// three decisions for the same reasons; it is restated rather than called for
/// [`number_for`]'s reason.
///
/// # Replace, keyed on the input
///
/// An edge is dropped and the new one appended, keyed on `(node, slot)` — the
/// node that declares the input and what its procedure calls it. It is forced
/// rather than chosen: `SetError::SlotBoundTwice` refuses two edges on one
/// input where the Set is built, so an append would make the *second* call on
/// an input a refusal and leave a model unable to change its mind.
///
/// The key does not include the deck slot, because the run's wiring does not.
/// [`App::edges`] is one list for the whole run and an edge naming a node a Set
/// has not got is passed over where the Set is built. So a request names a deck
/// slot to say *which slot rebuilds*, and two slots holding a node of the same
/// name share one entry in this list.
///
/// # A slot this deck does not hold
///
/// Refused, in [`karakuri_environment::no_such_slot`]'s words, and nothing is
/// rewired — the decision [`App::save_set`] already makes and for its reason:
/// the server checks the number against its own `Slots` before it sends, and
/// this is the guard that does not depend on it having.
///
/// # The same input wired twice on one frame
///
/// Every request is applied, in the order it arrived, and the last one is what
/// the run is wired with. One aim per slot goes out after all of them are in
/// the list, so the rebuild carries the settled wiring rather than an
/// intermediate one. A request the same frame overwrote is told so: its edge
/// *was* written and then replaced, and a reply saying only "wired" would be a
/// true sentence about a state the run no longer holds.
pub(crate) fn rewired(
    asked: &[(usize, karakuri_engine::set::Edge)],
    edges: &mut Vec<karakuri_engine::set::Edge>,
    aims: &mut [Aiming],
    slot_count: usize,
) -> Vec<Result<String, String>> {
    use std::fmt::Write as _;
    // **Every edge into the list before any watcher is re-aimed**, so that a
    // frame carrying two of them rebuilds once, at the wiring the frame ended
    // with.
    let mut said: Vec<Option<Result<String, String>>> = asked.iter().map(|_| None).collect();
    let mut named: Vec<usize> = Vec::new();
    for (at, (slot, edge)) in asked.iter().enumerate() {
        if !slot_in_range(*slot, slot_count) {
            said[at] = Some(Err(format!(
                "{}, and nothing was rewired",
                karakuri_environment::no_such_slot(*slot, slot_count)
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
        // Keyed on the input alone, like the replacement above, and **only ones
        // that were applied**: a later request refused for its slot number wrote
        // nothing, and telling this one it had been replaced by an edge that
        // never landed would be the same lie in the other direction.
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

/// One slot, with a worker watching its own two files behind it — which is
/// what puts a candidate in the Staging lane and is the whole of what that
/// took.
///
/// # It is `karakuri-cli`'s wiring and deliberately not a second one
///
/// That program builds every `--watch` slot as `HotSwap::new` over a
/// `watch::Watch`, and every argument below is the reading `Set::build` made
/// at startup restated, because that is what a rebuild is: the *material*
/// changed and nothing else about the slot did. A request that derived any of
/// them again would be a slot that comes back as a different Set on the first
/// save — which is the failure `Watch`'s own fields are each documented
/// against.
///
/// - `Layering::Overdraw` and no `live` — `Set::build`'s own, which is
///   what every slot was built with: one target, however many renderers.
/// - No `capacity` — so each geometry is rebuilt at the capacity it
///   declares, which is [`capacity_of`]'s line asked again on the worker.
///   This program has no `--capacity` to override it (see [`USAGE`]), and
///   passing the startup reading would pin the slot to a declaration the file
///   may have just changed.
/// - The slot's own salt, and no per-source salts — `Set::build` passes
///   `&[]` and says why: *"A pair assigns nothing, so the one source is salted
///   from the Set's seed and its ordinal — which for source 0 is that seed
///   unchanged."* So a rebuild is the same simulation of new material rather
///   than a new one, and every slot stays at the salt [`slot_salt`] counted
///   off for it.
/// - The default camera — `Request::camera` is an `Orbit` rather than an
///   `Option` because *"a Set holds a built-in camera whatever its files
///   declare"*, and this program loads none, so the default is what it is
///   running.
/// - No overrides, no published controls, no bindings, no edges and no
///   authorities — this program has no flag for any of the five and grants
///   nothing (ADR-0216), so each is the empty list the startup build used.
///
/// # The store, and the two things a watcher is given
///
/// `Watch::storing_to` puts every build's sources in the store and reports them
/// as [`watch::Built`], which is where a rebuilt node's address comes from —
/// and nothing could derive one until something needed one, which is *Keep what
/// a deck is playing*: a Set file references its sources by hash, so a slot
/// whose builds were never stored is a slot that cannot be written down. See
/// [`Playing`], which is the other end of that channel.
///
/// `Watch::snapshotting_to` keeps every version that compiled under
/// `<store>/history/`, so an edit can be walked back — a hand at an editor and
/// a model writing over MCP both reach a file through the same path, and this
/// is where the version they replaced is kept (P-0096, ADR-0089). The whole
/// run shares one `history::Snapshots` with the launch-time seed, or the
/// first rebuild files the untouched procedure a second time.
///
/// Every slot launches under no Set, which is the truth rather than a
/// placeholder: this program opens on a pair, and a pair somebody typed is not
/// a Set (ADR-0276). What turns that into an id is a library load — [`loading`]
/// sends the id on the aim, and the watcher moves it — so the versions written
/// after a load are filed under the Set that was loaded.
///
/// # What it costs the frame path, which is nothing
///
/// A worker thread per slot, polling the two files every hundred milliseconds
/// and compiling on that thread. The render thread's side is unchanged:
/// `install_if_ready` polls the same channel with `try_recv` whether the
/// `Sender` is live or was dropped at construction, and a swap has always
/// landed at a frame boundary (ADR-0005). What is new on a *frame* is a
/// build's install, which is the mechanism this deck was already built on.
// **Eight, and each is a distinct thing this slot's watcher needs**: a device, a
// pair, a live Set, which slot it is, its salt, where builds go, where its
// layout is published and where its versions are kept. A struct bundling them
// would be one type with one construction site and one reader.
#[allow(clippy::too_many_arguments)]
pub(crate) fn watched(
    gpu: &Gpu,
    sources: &Sources,
    live: Set,
    slot: usize,
    salt: u32,
    // Where this watcher puts what it builds, and where it says so — or `None`
    // for a harness with no store to write into. See [`Engine::new`].
    stored: Option<(std::sync::Arc<Store>, std::sync::mpsc::Sender<watch::Built>)>,
    // **The run's one published layout**, made in [`main`] beside the opening
    // and for the same reason — see [`Aiming::pointing`]. This slot's row of it
    // is written here, at construction, and again on every re-point.
    pointing: karakuri_mcp::Slots,
    // **The run's one history**, seeded in [`main`] from the same files this
    // slot watches, and `None` for a harness with no store — the same
    // condition `stored` above is `None` under, and a separate argument
    // because the two keep different things: that one is what reached the
    // *screen* and this is what reached the *compiler*. A version that cost
    // too much to run is in both — it is in the slot, stopped — and a version
    // that compiled and was superseded before it landed is in this alone.
    snapshots: Option<history::Shared>,
) -> (HotSwap, Aiming) {
    // **The other end of `Watch::aimed_by`**, kept by [`Engine`] so that a
    // load can say *look at these files instead*. It is made here rather than
    // by the caller because the watcher it belongs to is made here, and a
    // sender paired with the wrong slot's watcher would load a deck the
    // operator did not name.
    let (aim, aimed) = std::sync::mpsc::channel();
    // **The aim this watcher is constructed with, said once in a value rather
    // than only in the argument list below.** It is `Watch::new`'s arguments
    // less the slot, which is what [`watch::Aim`] is, and it is kept so that a
    // rewiring can restate the twelve fields it does not change. The two lists
    // are read side by side here on purpose: a field that disagreed would be a
    // rewiring that quietly moved something else.
    let at = watch::Aim {
        head: karakuri_environment::compile::Named::bare(&sources.l1),
        rest: vec![karakuri_environment::compile::Named::bare(&sources.l4)],
        layering: Layering::Overdraw,
        live: None,
        capacity: None,
        seed_salt: salt,
        salts: Vec::new(),
        camera: karakuri_engine::camera::Orbit::default(),
        overrides: Vec::new(),
        published: Vec::new(),
        bindings: Vec::new(),
        edges: Vec::new(),
        authorities: Vec::new(),
        // **No Set, because a slot launches on the pair this program was
        // started with and a pair somebody typed is not a Set.** The nearest
        // thing to a name is [`Sources::material`], which is a readout for the
        // mixer strip — filing versions under it would put rows in the history
        // under a Set no listing can ever match (ADR-0276). [`loading`] is what
        // turns this into an id.
        set: None,
    };
    let watching = watch::Watch::new(
        slot,
        // **Bare, so every node is called what its procedure declares**,
        // which is `Set::build`'s own: *"A pair names nothing, so both
        // nodes are called what their procedures are."* A name here
        // belongs to the *use* and this program has no syntax for one.
        karakuri_environment::compile::Named::bare(&sources.l1),
        vec![karakuri_environment::compile::Named::bare(&sources.l4)],
        Layering::Overdraw,
        None,
        None,
        salt,
        Vec::new(),
        karakuri_engine::camera::Orbit::default(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .aimed_by(aimed);
    // **Where a rebuild's sources go**, so that what a slot is running has an
    // address a Set file can name. Nothing is put until a build happens, and
    // the put is on the worker thread that compiled it.
    let watching = match stored {
        Some((store, tx)) => watching.storing_to(store, tx),
        None => watching,
    };
    // **Where every version that compiles is kept**, under the Set this slot is
    // running — which at launch is none, and is `at.set` for the same reason
    // the CLI reads its own aim there: one answer, and the aim is what moves it.
    let watching = match snapshots {
        Some(shared) => watching.snapshotting_to(shared, at.set.clone()),
        None => watching,
    };
    let swap = HotSwap::new(
        &gpu.device,
        &gpu.queue,
        live,
        // **The engine's own default rather than a number written here**: a
        // budget transcribed into this file would be a second answer to *how
        // long may a frame take* the day the engine's moves (ADR-0179 on a
        // number that is not even the mock's). It is 20 ms, which is 60 Hz
        // with room, and it is what makes the budget's verdict reachable in
        // this program at all — `HotSwap::fixed` judged against infinity, so no
        // slot could ever be stopped for cost.
        //
        // **And it is the opening value rather than the final one.** A
        // `HotSwap` is built here, from the launch pair, before there is a
        // window to ask what the display's interval is; `App::resumed` reads
        // `budget_ms(&window)` the moment there is one and narrows every slot
        // through `Deck::set_frame_budget_ms`. The constant is what stands where
        // the platform names no refresh rate (ADR-0313).
        DEFAULT_BUDGET_MS,
        Box::new(watching),
    );
    (swap, Aiming::new(aim, at, pointing, slot))
}
