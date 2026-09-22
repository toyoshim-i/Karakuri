use karakuri_engine::chain_swap::ChainSlot;
use karakuri_engine::master::{Chain, Cut, Slot, SlotSpec};

use super::shipped;

/// Resolve and check every slot of a described chain, holding no device.
///
/// `resolve` answers what an address's source is — the shipped three without a
/// store, anything else out of one — and a slot whose address nothing holds is
/// refused with the address in the message, which is what ADR-0340 asks of a
/// replay meeting a procedure the store does not have. A source that does not
/// check is refused with the slot's position and its address.
///
/// The one derivation of a chain's [`ChainSlot`]s: [`build_chain`] compiles
/// these against a device on the calling thread, and [`apply_chain`] hands them
/// to the chain worker.
pub fn check_chain(
    slots: &[SlotSpec],
    resolve: &dyn Fn(&str) -> Option<String>,
) -> Result<Vec<ChainSlot>, String> {
    let sources = resolve_chain(slots, resolve)?;
    let mut checked = Vec::with_capacity(slots.len());
    for (at, (spec, source)) in slots.iter().zip(&sources).enumerate() {
        checked.push(ChainSlot {
            spec: spec.clone(),
            checked: crate::compile::check(source)
                .map_err(|e| format!("master chain slot {at}: {}: {e}", spec.procedure))?,
        });
    }
    Ok(checked)
}

/// Compile a described chain into one the engine can run, on the calling
/// thread.
///
/// Creates a shader module and a render pipeline per slot, which is why the
/// real-time hosts do not call this: they call [`apply_chain`], which does the
/// same work on `karakuri-chain`. This is the synchronous path — the offline
/// renderer and replay, where no frame is waiting (ADR-0354).
///
/// The chain it returns carries no targets, so the `Present` it is installed on
/// allocates them.
pub fn build_chain(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    slots: &[SlotSpec],
    resolve: &dyn Fn(&str) -> Option<String>,
) -> Result<Chain, String> {
    let checked = check_chain(slots, resolve)?;
    let mut built = Vec::with_capacity(checked.len());
    for (at, want) in checked.iter().enumerate() {
        built.push(
            Slot::build(
                device,
                layout,
                want.spec.procedure.clone(),
                &want.checked,
                want.spec.cut,
                want.spec.params.clone(),
            )
            .map_err(|e| format!("master chain slot {at}: {e}"))?,
        );
    }
    Ok(Chain::new(built))
}

/// The source behind every slot of a described chain, in order, or the one
/// refusal for an address nothing holds.
///
/// Holds no device, so whether a run can resolve a chain at all is settled
/// before anything is compiled: the same question is answered the same way on
/// the frame path, at replay, and anywhere a chain is checked before it is
/// installed. The refusal names the slot's position and its address (ADR-0340),
/// and it refuses at the first such slot.
///
/// `resolve` is [`resolve_procedure`] bound to whatever store the caller has.
pub fn resolve_chain(
    slots: &[SlotSpec],
    resolve: &dyn Fn(&str) -> Option<String>,
) -> Result<Vec<String>, String> {
    slots
        .iter()
        .enumerate()
        .map(|(at, spec)| {
            resolve(&spec.procedure).ok_or_else(|| {
                format!("master chain slot {at}: nothing holds `{}`", spec.procedure)
            })
        })
        .collect()
}

/// What an address resolves to, for [`resolve_chain`], [`build_chain`] and
/// [`apply_chain`] — the one resolution every host uses.
///
/// The shipped three first and without a store at all — a windowed run that has
/// never saved anything can still put a preset in its chain — and then whatever
/// store the caller has. A store is optional and that is the point: a run
/// recording nothing creates nothing (`Placed::put`'s own division).
pub fn resolve_procedure(
    store: Option<&karakuri_store::store::Store>,
    address: &str,
) -> Option<String> {
    if let Some(source) = shipped::source(address) {
        return Some(source.to_string());
    }
    let hash: karakuri_store::hash::Hash = address.parse().ok()?;
    let bytes = store?.get_artifact(&hash).ok()?;
    String::from_utf8(bytes).ok()
}

/// Put a described chain on a `Present` without compiling anything on the
/// calling thread.
///
/// Two paths, and which one is taken is P-0091's question rather than a
/// convenience. `Record::MasterChain` is written whole — a stream that moved
/// one slot without saying where the others stood describes a chain a replay
/// cannot put back — so the ordinary case of applying one is a record whose
/// *shape* is the shape already running with one number different. That is a
/// `queue.write_buffer` per slot and nothing else, and it returns having
/// applied it.
///
/// A record whose shape differs is a build, and the build is asked for here and
/// happens on `karakuri-chain`: sources are resolved and checked on this thread
/// — neither touches a device — and the procedures, the pipelines and the
/// targets are made on the worker. Until the build lands the chain that is
/// running keeps drawing and `Present::chain_spec` still reads it, so a surface
/// that draws the chain draws the outgoing list until the frame the new one is
/// installed on. Calling again with a list already being built is a no-op, so a
/// host may ask on every frame.
///
/// `Ok` means the list was applied or a build was asked for, not that a build
/// succeeded: a slot that refuses at compile time is a
/// [`ChainEvent::Refused`](karakuri_engine::ChainEvent) on `swap`, and the
/// chain keeps what it had. `Err` is a slot whose address nothing holds or
/// whose source does not check, and then nothing was asked for.
///
/// [`ChainSwap::begin_frame`](karakuri_engine::ChainSwap::begin_frame) is what
/// installs the result, and must be called before the frame's encoder exists.
pub fn apply_chain(
    swap: &mut karakuri_engine::ChainSwap,
    present: &mut karakuri_engine::Present,
    queue: &wgpu::Queue,
    slots: &[SlotSpec],
    resolve: &dyn Fn(&str) -> Option<String>,
) -> Result<(), String> {
    let shape: Vec<(String, Option<Cut>)> =
        slots.iter().map(|s| (s.procedure.clone(), s.cut)).collect();
    let params: Vec<std::collections::BTreeMap<String, f32>> =
        slots.iter().map(|s| s.params.clone()).collect();
    if present.set_chain_params(queue, &shape, &params) {
        return Ok(());
    }
    if swap.is_building(slots) {
        return Ok(());
    }
    swap.request(present, check_chain(slots, resolve)?);
    Ok(())
}

/// Put a described chain on a `Present`, compiling it on the calling thread.
///
/// The synchronous path, and [`karakuri_engine::HotSwap::install`] is its
/// counterpart one layer down: a run with no frame waiting on the clock — the
/// offline renderer, a replay — builds where it stands rather than carrying a
/// worker. The cheap path is the same one [`apply_chain`] takes and for the
/// same reason.
///
/// Returns having applied the list or having refused it. A refusal names the
/// slot and the chain keeps what it had.
pub fn install_chain(
    present: &mut karakuri_engine::Present,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    slots: &[SlotSpec],
    resolve: &dyn Fn(&str) -> Option<String>,
) -> Result<(), String> {
    let shape: Vec<(String, Option<Cut>)> =
        slots.iter().map(|s| (s.procedure.clone(), s.cut)).collect();
    let params: Vec<std::collections::BTreeMap<String, f32>> =
        slots.iter().map(|s| s.params.clone()).collect();
    if present.set_chain_params(queue, &shape, &params) {
        return Ok(());
    }
    let chain = build_chain(device, present.chain_layout(), slots, resolve)?;
    drop(present.set_chain(device, queue, chain));
    Ok(())
}

/// The master chain that is running, as the reading the conversion needs.
///
/// [`current_look`]'s function one pass along: an operation names one slot of
/// the chain and `Record::MasterChain` carries the whole list, so the list that
/// is running is what completes the record. See
/// `docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`.
pub fn current_chain(slots: &[SlotSpec]) -> karakuri_operation_record::Chain {
    karakuri_operation_record::Chain {
        slots: slots
            .iter()
            .map(|s| karakuri_store::record::ChainSlot {
                procedure: s.procedure.clone(),
                cut: s.cut.map(|c| c.name().to_string()),
                params: s.params.clone(),
            })
            .collect(),
    }
}

/// What one `kind L5` source offers a chain, or `None` for a source that is not
/// one: the content address a slot of it is named by, and whether it declares
/// `retains`.
///
/// A surface offering a procedure to the chain needs both: an add carries the
/// address and carries a cut exactly where the procedure declares `retains`
/// (`docs/adr/0348-a-chain-slots-cut-is-set-through-the-parameter-row.md`). Both
/// are facts about the file rather than about the store it came from, so this
/// takes the source and not a path.
///
/// It checks rather than scanning for a word: `retains` is a header declaration
/// of the language and `check` is the one reader of it.
pub fn l5_offer(source: &str) -> Option<(String, bool)> {
    let checked = crate::compile::check(source).ok()?;
    (checked.kind == karakuri_ir::Kind::L5).then(|| (shipped::address(source), checked.retains))
}
