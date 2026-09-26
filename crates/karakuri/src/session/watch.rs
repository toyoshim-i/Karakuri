use crate::{slot_in_range, Aiming, Sources};
use karakuri_engine::set::Layering;
use karakuri_engine::{Gpu, HotSwap, Set, DEFAULT_BUDGET_MS};
use karakuri_environment::{history, watch};
use karakuri_store::store::Store;

/// Applies and settles wire edge requests arriving within a single frame.
///
/// Validates slots, updates the wiring list replacing existing inputs,
/// re-aims affected watchers once per slot, and formats descriptive replies.
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

/// Constructs a background file watcher and hot-swap bridge for a single deck slot.
///
/// Configures background recompilation, store source archival, and version history
/// tracking without blocking the frame render thread (ADR-0005, ADR-0089, P-0096).
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
    // The run's published layout updated at construction and on re-point.
    pointing: karakuri_mcp::Slots,
    // Compiler snapshot history shared across the run, or None without store.
    snapshots: Option<history::Shared>,
) -> (HotSwap, Aiming) {
    // Channel for re-aiming this slot's watcher on procedure or set load.
    let (aim, aimed) = std::sync::mpsc::channel();
    // Initial aim descriptor preserved for subsequent re-wiring updates.
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
        // Launch pair is not a named set; populated upon explicit library load (ADR-0276).
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
        // Engine default frame budget in ms, narrowed once window refresh is known (ADR-0179, ADR-0313).
        DEFAULT_BUDGET_MS,
        Box::new(watching),
    );
    (swap, Aiming::new(aim, at, pointing, slot))
}
