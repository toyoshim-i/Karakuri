use std::sync::mpsc;

use karakuri_ir::{Diagnostic, DiagnosticReport};
use karakuri_operation::Operation;
use karakuri_store::hash::Hash;
use karakuri_store::record::{Layer, Record};
use karakuri_store::store::{Store, StoreError};
use serde_json::Value;

use super::super::*;
use super::*;

/// Verify a set configuration in the store and return a `DiagnosticReport`.
pub fn check_set_configuration(store_path: &std::path::Path, id: &str) -> DiagnosticReport {
    let store = match Store::open(store_path) {
        Ok(s) => s,
        Err(e) => {
            return DiagnosticReport {
                diagnostics: vec![Diagnostic {
                    code: "KIR-E501-STORE-OPEN-FAILED".to_string(),
                    message: format!("cannot open store at `{}`: {e}", store_path.display()),
                    line: None,
                    column: None,
                    remedy: Some(
                        "Verify the store directory path exists and has correct permissions."
                            .to_string(),
                    ),
                }],
                success: false,
            };
        }
    };
    match store.read_set(id) {
        Ok(_) => DiagnosticReport::ok(),
        Err(e) => DiagnosticReport {
            diagnostics: vec![Diagnostic {
                code: "KIR-E502-SET-READ-FAILED".to_string(),
                message: format!("failed to read set `{id}`: {e}"),
                line: None,
                column: None,
                remedy: Some(
                    "Check that the set ID is correctly spelled and saved in the library."
                        .to_string(),
                ),
            }],
            success: false,
        },
    }
}

/// Parses `save_set` arguments into an `Operation::SaveSet`.
pub(crate) fn kept(args: &Value, slots: &Slots) -> Result<Operation, String> {
    let slot = args
        .get("slot")
        .and_then(Value::as_u64)
        .ok_or("`slot` is required and is a number")? as usize;
    let deck = deck_named(slot, slots)?;
    let id = match args.get("id") {
        None | Some(Value::Null) => None,
        Some(id) => Some(checked_id(
            id.as_str()
                .ok_or("`id` is a string: what to file the set under")?,
        )?),
    };
    Ok(Operation::SaveSet { deck, id })
}

/// Parses `read_set` arguments into an `Operation::ReadSet`.
pub(crate) fn named_set(args: &Value) -> Result<Operation, String> {
    let id = args
        .get("id")
        .and_then(Value::as_str)
        .ok_or("`id` is required and is a string: which set to read")?;
    Ok(Operation::ReadSet {
        id: checked_id(id)?,
    })
}

/// Parses `list_sets` arguments into an `Operation::ListSets`.
pub(crate) fn listing(args: &Value) -> Result<Operation, String> {
    let holds = match args.get("holds") {
        // `null` is absent, for the reason [`kept`]'s `id` says: a client
        // building arguments from a record with an empty field sends one, and
        // that is a caller saying nothing rather than a caller getting a type
        // wrong.
        None | Some(Value::Null) => None,
        Some(value) => Some(
            value
                .as_str()
                .ok_or("`holds` is a string: part of a node's name")?
                .to_string(),
        ),
    };
    let layer = match args.get("layer") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let spelled = value
                .as_str()
                .ok_or("`layer` is a string: which layer a set must hold a node on")?;
            // **The same spellings the other tools take**, from the same table:
            // a model that addressed `Field` in `read_procedure` must not be
            // told there is no such layer here.
            let kind = layer_named(spelled)
                .ok_or_else(|| format!("no layer `{spelled}` — {}", layer_list()))?;
            Some(layer_of(kind))
        }
    };
    Ok(Operation::ListSets { holds, layer })
}

/// [`Operation::SaveSet`]: requests the render loop to save current slot state.
pub(crate) fn save_set(
    deck: u8,
    id: Option<&str>,
    state: &State,
) -> Result<mpsc::Receiver<News>, String> {
    let slot = usize::from(deck);
    let id = id.map(str::to_string);
    let (tx, rx) = mpsc::channel();
    state
        .asked
        .try_send(SaveRequest {
            slot,
            id,
            reply: Reply(tx),
        })
        // **Answered rather than waited for**, both ways. A model must never be
        // left holding a call on a loop that will not answer it, and these are
        // the two shapes of "it will not": one that has stopped taking requests,
        // and one that is gone.
        .map_err(|e| match e {
            // Full queue indicates requests are queued waiting on render thread progress.
            mpsc::TrySendError::Full(_) => format!(
                "the render loop has {ASKED} save requests queued and no room for another: \
                 it is taking them slower than they are arriving, or it is not running \
                 frames at all. Nothing was saved, and asking again is safe"
            ),
            mpsc::TrySendError::Disconnected(_) => {
                "the render loop has ended: this run is shutting down and nothing was saved"
                    .to_string()
            }
        })?;
    Ok(rx)
}

/// Reads metadata, declared parameters, and element memory estimates for a saved Set by ID.
///
/// Preconditions: `id` must be a valid sanitized Set identifier.
/// Queries persistent store metadata cards without requiring compilation or GPU initialization.
pub(crate) fn read_set(id: &str, state: &State) -> Result<String, String> {
    // Already one path component, because that is part of naming a set rather
    // than part of reading one — see [`asked`]'s `read_set` arm and
    // [`checked_id`].
    let store = Store::open(&state.store)
        .map_err(|e| format!("the store at `{}`: {e}", state.store.display()))?;
    let lines = store.read_set(id).map_err(|e| {
        format!(
            "reading set `{id}`: {e} — this reads the operator's library, which is \
             filed under the id a set was saved under by the `k` key or by \
             `--save-set ID`. A set kept with `save_set` is in `<store>/{}/` and not \
             here, because the library is written by the operator's own act",
            Store::SANDBOX
        )
    })?;
    // **The file's own order**, which is the order [`crate::setfile::save`]
    // wrote the nodes in, and the order a hand-written file chose. Sorting by
    // layer would impose a reading nobody wrote, for the reason
    // [`crate::meta::card`] keeps a procedure's parameters in declaration order.
    let nodes: Vec<(Layer, u32, Option<String>, Hash)> = lines
        .iter()
        .filter_map(|line| match line.record() {
            Record::Slot {
                at,
                name,
                proc_hash,
            } => Some((at.layer, at.index, name.clone(), *proc_hash)),
            _ => None,
        })
        .collect();
    if nodes.is_empty() {
        return Ok(format!(
            "set `{id}` is in this store and names no material: it holds {} record{} \
             and none of them is a `slot`, so there is nothing in it to describe.",
            lines.len(),
            if lines.len() == 1 { "" } else { "s" },
        ));
    }
    let mut out = format!(
        "set `{id}` holds {} node{}, and `--load-set {id}` plays it. Everything below \
         is what a procedure *declares* — the range a value is refused outside of — \
         and not what this set has anything turned to.\n",
        nodes.len(),
        if nodes.len() == 1 { "" } else { "s" },
    );
    for (layer, index, name, hash) in nodes {
        out.push('\n');
        out.push_str(&node_block(&store, layer, index, name.as_deref(), &hash));
    }
    out.push('\n');
    out.push_str(&element_storage_block(&store, id));
    Ok(out)
}

/// Maximum number of Sets returned in a single list response.
pub const LISTED: usize = 20;

/// Lists Sets stored in the library, ordered by modification time descending.
pub(crate) fn list_sets(
    holds: Option<&str>,
    layer: Option<karakuri_operation::Layer>,
    state: &State,
) -> Result<String, String> {
    // Performs case-insensitive matching against requested procedure name.
    let holds = holds.map(str::to_ascii_lowercase);
    let layer = layer.map(karakuri_environment::meta::op_to_record);
    let opened = |e: StoreError| format!("the store at `{}`: {e}", state.store.display());
    let store = Store::open(&state.store).map_err(opened)?;
    let mut sets = karakuri_environment::setfile::summarise(&store).map_err(opened)?;
    let held = sets.len();
    // **An empty store is an answer and not a failure**, and it is a different
    // answer from a filter that matched nothing: one sends a reader to
    // `save_set`, the other to a different filter. Answered before the filters
    // are applied, because a filter over nothing has nothing to say.
    if held == 0 {
        return Ok(format!(
            "this store holds no sets at all — nothing has been kept here yet. This lists \
             the operator's library, which is written by their own act: the `k` key, or \
             `--save-set ID` on the command line. What `save_set` keeps goes to the \
             sandbox and is not listed here. This store is `{}`.",
            state.store.display()
        ));
    }
    sets.sort_by(|a, b| b.written.cmp(&a.written).then_with(|| a.id.cmp(&b.id)));
    // Evaluates filter criteria across the entire Set rather than individual nodes.
    sets.retain(|set| {
        holds.as_ref().is_none_or(|holds| {
            set.nodes
                .iter()
                .any(|node| node.name.to_ascii_lowercase().contains(holds))
        }) && layer.is_none_or(|layer| set.nodes.iter().any(|node| node.layer == layer))
    });
    let narrowed = describe_filters(holds.as_deref(), layer);
    let matched = sets.len();
    if matched == 0 {
        return Ok(format!(
            "none of the {held} set{} {narrowed}. The store is not empty — \
             call this with no arguments to see everything in it. `holds` is matched \
             against what each node is called, which is the name the set gave it or the \
             name its procedure gives itself.",
            plural(held),
        ));
    }
    let shown = matched.min(LISTED);
    let mut out = if matched > shown {
        // **Never a truncated list that reads as a whole one.** A model told
        // "here are your sets" over twenty of two hundred will tell its user
        // they have twenty, and act on a library it has not seen.
        format!(
            "{matched} set{} {narrowed}, and the {shown} most recently written are below — \
             **this is not all of them**: {} more matched and are not listed. Narrow it \
             with `holds`, or with `layer`, or ask for a set by id with `read_set`.\n",
            plural(matched),
            matched - shown,
        )
    } else {
        format!(
            "{matched} set{} {narrowed}, most recently written first — all of them are \
             below.\n",
            plural(matched),
        )
    };
    for set in sets.iter().take(shown) {
        out.push_str(&set_line(set));
    }
    out.push_str(
        "\nEach line is a set's id, when it was written, and what it holds: an address \
         per node and what that node is called in this set. `read_set` with one of these \
         ids says what each of its procedures declares — the parameters, the element \
         counts and what it emits — and `--load-set ID` is what plays one.\n",
    );
    Ok(out)
}

/// One Set as a line of a listing.
fn set_line(set: &karakuri_environment::setfile::SetSummary) -> String {
    let written = karakuri_environment::setfile::written_at(set.written);
    // **A file in `sets/` that will not read is listed and named.** Dropping it
    // would answer "what have I kept" with something missing, and rendering it
    // as a set of no nodes would say it holds nothing.
    if let Some(why) = &set.unreadable {
        return format!(
            "`{}` — written {written}, and could not be read: {why}\n",
            set.id
        );
    }
    if set.nodes.is_empty() {
        return format!(
            "`{}` — written {written}, and names no material: it holds no `slot` record\n",
            set.id
        );
    }
    let nodes: Vec<String> = set
        .nodes
        .iter()
        .map(|node| {
            format!(
                "{}:{} `{}`",
                layer_spelled(node.layer),
                node.index,
                node.name
            )
        })
        .collect();
    format!(
        "`{}` — written {written}, {} node{}: {}\n",
        set.id,
        set.nodes.len(),
        plural(set.nodes.len()),
        nodes.join(", "),
    )
}

/// What the filters did to a listing, as the middle of a sentence — so that
/// every count this tool prints is said to be a count *of* something, and a
/// filtered answer can never be read as the whole store.
fn describe_filters(holds: Option<&str>, layer: Option<Layer>) -> String {
    match (holds, layer) {
        (None, None) => "in this store".to_string(),
        (Some(holds), None) => format!("in this store hold a node whose name contains `{holds}`"),
        (None, Some(layer)) => format!("in this store hold a {} node", layer_spelled(layer)),
        (Some(holds), Some(layer)) => format!(
            "in this store hold both a node whose name contains `{holds}` and a {} node",
            layer_spelled(layer)
        ),
    }
}
