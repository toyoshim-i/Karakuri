use karakuri_ir::typed::Checked;
use karakuri_operation::Operation;
use karakuri_store::hash::Hash;
use karakuri_store::ndjson::Line;
use karakuri_store::record::{Layer, Record};
use karakuri_store::store::{Store, StoreError};
use serde_json::Value;

use super::super::*;
use super::*;

pub(crate) fn swap_outcome(state: &mut State) -> Result<String, String> {
    state.drain();
    let dropped = state.dropped.load(std::sync::atomic::Ordering::Relaxed);
    let missing = if dropped == 0 {
        String::new()
    } else {
        format!(
            "\n\n({dropped} earlier report{} were dropped for want of room — this is \
                 not the whole history)",
            if dropped == 1 { "" } else { "s" }
        )
    };
    Ok(if state.recent.is_empty() {
        format!(
            "nothing has swapped, been overloaded or failed to build since this run \
             started.{missing}"
        )
    } else {
        format!("{}{missing}", state.recent.join("\n"))
    })
}

/// Parses arguments for `walk_history` into an `Operation::WalkHistory`.
pub(crate) fn walked_history(args: &Value) -> Result<Operation, String> {
    let set = args.get("set").and_then(Value::as_str).ok_or(
        "`set` is required and is a string: which Set's versions to walk. A walk is \
         narrowed to one Set — `list_sets` names the ids — because two decks playing one \
         Set have one history between them and a version written under no Set is matched \
         by no id",
    )?;
    Ok(Operation::WalkHistory {
        set: Some(checked_id(set)?),
    })
}

/// Maximum number of historical records to scan before narrowing by Set ID.
pub(crate) const WALKED: usize = 200;

/// Lists historical versions of a specified Set in reverse chronological order.
pub(crate) fn walk_history(set: Option<&str>, state: &State) -> Result<String, String> {
    // Cannot happen from this surface — [`walked_history`] requires `set` — and
    // written out rather than unwrapped, because the payload's `None` is a real
    // value on another surface and this is what it would mean here.
    let Some(id) = set else {
        return Err(
            "this walk names no Set, and a walk of no Set lists nothing: versions written \
             while a slot was running material nobody had saved are filed under no Set, and \
             no id matches them. Name a Set — `list_sets` says which ones this store holds"
                .to_string(),
        );
    };
    let found = karakuri_environment::history::list(&state.store, WALKED)?;
    let rows: Vec<&karakuri_environment::history::Version> = found
        .versions
        .iter()
        .filter(|version| version.set.as_deref() == Some(id))
        .collect();
    let mut out = String::new();
    if rows.is_empty() {
        out.push_str(&format!(
            "no version of set `{id}` is in this store's edit history. Every build that \
             compiles is filed there, so this is a Set nothing has been edited on in what \
             the walk covers — or one this store has never played. `read_set` says what \
             `{id}` holds and `list_sets` says what else is here.\n"
        ));
    } else {
        let shown = rows.len().min(LISTED);
        if rows.len() > shown {
            // Truncation notice indicates matching rows exceeded the display limit.
            out.push_str(&format!(
                "{} version{} of set `{id}` are in what this walk covered, and the {shown} \
                 most recent are below — **this is not all of them**: {} more matched and \
                 are not listed.\n",
                rows.len(),
                plural(rows.len()),
                rows.len() - shown,
            ));
        } else {
            out.push_str(&format!(
                "{} version{} of set `{id}`, most recent first — all of the ones this walk \
                 covered are below.\n",
                rows.len(),
                plural(rows.len()),
            ));
        }
        for version in rows.iter().take(shown) {
            out.push_str(&format!("`{}`\n", version.filed_as()));
        }
    }
    if found.stopped_short {
        out.push_str(
            "\nThe walk stopped with day directories unread, so this is part of what is \
             there rather than all of it: it asks for the last few hundred versions this \
             store wrote and narrows them to the Set afterwards, so a store several Sets \
             are being edited in shows fewer of each.\n",
        );
    }
    if found.unclaimed > 0 {
        out.push_str(&format!(
            "\n{} entr{} under `history/` that this layout does not claim {} passed over. A \
             day directory is a place an operator works in by hand — `rm -rf \
             history/2026/07` is this store's whole retention policy — so whatever else is \
             in there is theirs.\n",
            found.unclaimed,
            match found.unclaimed {
                1 => "y",
                _ => "ies",
            },
            match found.unclaimed {
                1 => "was",
                _ => "were",
            }
        ));
    }
    // Append explanatory footer only when rows are present.
    if !rows.is_empty() {
        out.push_str(
            "\nEach row is the name the store filed a version under: when it was written, \
             which slot, which node of that slot, and what the procedure called itself. To \
             put one back, call `operate` with `Put a node's previous version back` and \
             `{\"deck\": <the row's slot>, \"revision\": {\"picked\": \"<row>\"}}` — the \
             version's bytes are written over that node's working copy and built like any \
             other edit. The gate on this history is **compiling** and not landing, so a \
             version that cost too much to run is in here too.\n",
        );
    }
    Ok(out)
}

/// The `s` on a count, in the one place, because every sentence here has one.
pub(crate) fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// Computes and formats the element memory storage required by a Set, node by node.
pub(crate) fn element_storage_block(store: &Store, id: &str) -> String {
    // Follows the same pattern as `node_block` when cost estimates are unavailable.
    let unavailable = |why: &str| {
        format!(
            "element storage: not computed — {why}. This is the one figure here that \
             needs the set to hold together as a whole, because what a node allocates \
             depends on what reaches it; everything above is read off each artifact's \
             own card and stands on its own.\n"
        )
    };
    let loaded = match karakuri_environment::setfile::load(store, id) {
        Ok(loaded) => loaded,
        Err(why) => return unavailable(&why),
    };
    // Resolves element capacity for each geometry from explicit file configuration
    // or falls back to procedure defaults.
    let mut sources: Vec<(&Checked, u32)> = Vec::with_capacity(loaded.l1s.len());
    for (at, l1) in loaded.l1s.iter().enumerate() {
        match loaded
            .capacities
            .get(at)
            .copied()
            .flatten()
            .or_else(|| l1.capacity.map(|declared| declared.default))
        {
            Some(capacity) => sources.push((l1, capacity)),
            None => {
                return unavailable(&format!(
                    "`{}` declares no `capacity` and this set records none for it, so \
                     there is no element count to size anything against",
                    l1.name
                ))
            }
        }
    }
    let l2s: Vec<&Checked> = loaded.l2s.iter().collect();
    let l3s: Vec<&Checked> = loaded.l3s.iter().collect();
    let fields: Vec<&Checked> = loaded.fields.iter().collect();
    let l4s: Vec<&Checked> = loaded.l4s.iter().collect();
    let plan = match karakuri_engine::Set::validate(
        &sources,
        &l2s,
        &l3s,
        &fields,
        &l4s,
        // Reads merge record from file to validate compositing limits against stored configuration.
        loaded.layering,
        // A salt decides what the elements *are* and never how many bytes they
        // take, so the set's own is enough here and a source deriving one from
        // it changes nothing this block prints.
        loaded.salts.first().copied().flatten().unwrap_or_default(),
        &loaded.salts,
        karakuri_engine::set::Wiring {
            l1s: &loaded.names.l1s,
            l2s: &loaded.names.l2s,
            l3s: &loaded.names.l3s,
            l4s: &loaded.names.l4s,
            fields: &loaded.names.fields,
            edges: &loaded.edges,
        },
    ) {
        Ok(plan) => plan,
        Err(e) => return unavailable(&format!("this set does not build: {e}")),
    };

    let planned = plan.element_storage();
    let names = plan.node_names();
    let total: u64 = planned.iter().map(|p| p.storage.bytes).sum();
    let mut out = format!(
        "element storage: {total} bytes in total, across the {} node{} of this set that \
         hold elements, at the capacities the file records. Nothing was built to find \
         that out: it is the arithmetic the allocation itself is sized by, run over the \
         file.\n",
        planned.len(),
        if planned.len() == 1 { "" } else { "s" },
    );
    for entry in &planned {
        out.push_str(&format!(
            "  `{}` — {} bytes for {} element{}, {} bytes each\n",
            names[entry.node],
            entry.storage.bytes,
            entry.storage.capacity,
            if entry.storage.capacity == 1 { "" } else { "s" },
            entry.storage.per_element(),
        ));
    }
    // Multiple nodes may instantiate the same procedure across different geometries.
    if planned
        .iter()
        .enumerate()
        .any(|(at, entry)| planned[..at].iter().any(|seen| seen.node == entry.node))
    {
        out.push_str(
            "A name appears twice above because this set has more than one geometry: the \
             chain is instantiated once per geometry, and each instance holds buffers of \
             its own.\n",
        );
    }
    // Renderers draw from upstream buffers without allocating separate element storage rows.
    out.push_str(
        "What that covers: one element struct per element, the four-byte liveness flag \
         beside it, the second copy a geometry keeps so it can read what it wrote last \
         step, and the destination index a geometry that spawns or kills pays for. A \
         renderer and a camera hold no elements and so have no row. It is NOT what this \
         set costs a GPU: render targets, uniform blocks and every other buffer not \
         indexed by an element are outside it, so it is a floor on device memory and \
         never the figure to allocate against. What it is exactly is the cost of one \
         more element — the per-element numbers above are exact divisions rather than \
         averages.\n",
    );
    out
}

/// One node of a Set: its address in the Set, its artifact, and its card.
pub(crate) fn node_block(
    store: &Store,
    layer: Layer,
    index: u32,
    name: Option<&str>,
    hash: &Hash,
) -> String {
    // Truncates artifact hash to 12 lowercase hex characters for display.
    let short = hash.short(12);
    let address = format!("{}:{index}", layer_spelled(layer));
    // The head of one block: the address, what the node is called, and the
    // address its source is stored under — except where the name *is* that
    // address, which is what a node with no name of its own and no card to
    // declare one gets, and saying it twice adds nothing to saying it once.
    let head = |called: &str| {
        if called == short {
            format!("{address} `{short}`")
        } else {
            format!("{address} `{called}` — stored as {short}")
        }
    };
    match store.read_meta(hash) {
        Ok(card) => {
            let (declared, body) = rendered_card(&card);
            let called =
                karakuri_environment::setfile::node_called(name, declared.as_deref(), hash);
            // Reports both the Set's internal node name and the procedure's source identifier.
            let also = match &declared {
                Some(declared) if *declared != called => {
                    format!(", and the artifact calls itself `{declared}`")
                }
                _ => String::new(),
            };
            format!("{}{also}\n{body}", head(&called))
        }
        // Missing metadata is expected for uncompiled artifacts; verifies existence before reporting missing.
        Err(StoreError::NotFound(_)) => {
            let standing = if store.get_artifact(hash).is_err() {
                "this store does not hold that artifact at all, so nothing here can \
                 say what it declares and `--load-set` could not build this set \
                 either — the set was saved somewhere else, or beside a store that \
                 has since been moved"
            } else {
                "its source is here and it has no metadata card. That is an ordinary \
                 state and not a damaged store: a card is derived rather than kept, so \
                 an artifact stored as bytes, or stored by a build older than cards, \
                 has none until something compiles it and stores it again. What it \
                 declares is in its source, at the top of the procedure"
            };
            // No card, so there is no declared name to weigh: the set's own
            // name if it has one, and the short hash otherwise.
            let called = karakuri_environment::setfile::node_called(name, None, hash);
            format!("{}\n  {standing}.\n", head(&called))
        }
        // A card that is there and will not read is the one case that *is* a
        // damaged store, and it says so in different words for that reason.
        Err(e) => {
            let called = karakuri_environment::setfile::node_called(name, None, hash);
            format!("{}\n  its card could not be read: {e}\n", head(&called))
        }
    }
}

/// Formats the metadata records of an artifact card into declared name and description body.
pub(crate) fn rendered_card(card: &[Line]) -> (Option<String>, String) {
    let mut declared = None;
    let mut body = String::new();
    let mut params = 0usize;
    for line in card {
        match line.record() {
            Record::Meta { name, .. } => declared = Some(name.clone()),
            Record::ParamDecl {
                key,
                ty,
                min,
                max,
                default,
            } => {
                params += 1;
                body.push_str(&format!(
                    "  param {key} : {ty}, anywhere from {min} to {max}{}\n",
                    match default {
                        Some(default) => format!(", and {default} until something turns it"),
                        None => ". Its default is an expression rather than a literal, so the \
                             card cannot state it as a number"
                            .to_string(),
                    }
                ));
            }
            Record::CapacityDecl { min, max, default } => body.push_str(&format!(
                "  capacity: between {min} and {max} elements, and {default} of them \
                 until a set says otherwise\n"
            )),
            Record::Emit { attrs } => body.push_str(&format!(
                "  emits {} — what a renderer drawn over it can consume\n",
                attrs.join(", ")
            )),
            _ => {}
        }
    }
    if params == 0 {
        body.push_str("  no parameters: there is nothing to turn on this one\n");
    }
    (declared, body)
}

/// Converts a record [`Layer`] to its human-readable protocol string name.
pub(crate) fn layer_spelled(layer: Layer) -> &'static str {
    LAYERS
        .iter()
        .copied()
        .find(|kind| karakuri_environment::meta::layer_of(*kind) == layer)
        .map_or("unknown", layer_name)
}
