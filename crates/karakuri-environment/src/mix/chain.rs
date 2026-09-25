use karakuri_engine::chain_swap::ChainSlot;
use karakuri_engine::master::{Chain, Cut, Slot, SlotSpec};

use super::shipped;

/// Resolves and checks every slot of a described chain without holding a device.
///
/// Returns an error naming the slot position and procedure address if an address
/// cannot be resolved or fails compilation checking.
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

/// Compiles a described chain into one the engine can run, on the calling thread.
///
/// Intended for synchronous offline rendering and replay paths.
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

/// Resolves the source code for every slot of a described chain in order.
///
/// Fails with the slot position and procedure address on the first unresolved slot.
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

/// Resolves an address to source code, checking shipped presets before querying the store.
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

/// Puts a described chain on a `Present`, updating parameters immediately or requesting an async build.
///
/// If slot count and procedure IDs match, parameters are updated directly on the queue.
/// Otherwise, an asynchronous chain compilation is requested on the chain worker thread.
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

/// Puts a described chain on a `Present`, compiling synchronously on the calling thread.
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

/// Converts a slice of [`SlotSpec`] into an operation record [`Chain`].
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

/// Validates whether a source is `kind L5` and returns its address and `retains` flag.
pub fn l5_offer(source: &str) -> Option<(String, bool)> {
    let checked = crate::compile::check(source).ok()?;
    (checked.kind == karakuri_ir::Kind::L5).then(|| (shipped::address(source), checked.retains))
}
