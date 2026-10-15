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
                diagnostics: vec![Diagnostic::new(
                    karakuri_ir::error::codes::E501_STORE_OPEN_FAILED,
                    format!("cannot open store at `{}`: {e}", store_path.display()),
                    None,
                    None,
                    Some(
                        "Verify the store directory path exists and has correct permissions."
                            .to_string(),
                    ),
                )],
                success: false,
                hint: None,
            };
        }
    };
    match store.read_set(id) {
        Ok(_) => DiagnosticReport::ok(),
        Err(e) => DiagnosticReport {
            diagnostics: vec![Diagnostic::new(
                karakuri_ir::error::codes::E502_SET_READ_FAILED,
                format!("failed to read set `{id}`: {e}"),
                None,
                None,
                Some(
                    "Check that the set ID is correctly spelled and saved in the library."
                        .to_string(),
                ),
            )],
            success: false,
            hint: None,
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
            // Resolve layer name using shared vocabulary spelling table.
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
        // Fail promptly if the channel is full or disconnected rather than blocking.
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
/// Queries persistent store metadata cards and full procedure sources without requiring compilation or GPU initialization.
pub(crate) fn read_set(id: &str, state: &State) -> Result<String, String> {
    let gate = state.opening.read().library();
    if gate == karakuri_operation::gate::LibraryGate::Off {
        return Err(
            "the Library gate is off — the operator opens it at the head of the Library bay".into(),
        );
    }

    // 1. Check presets first
    let presets_dir = super::library::resolve_presets_dir();
    let preset_path = presets_dir.join(format!("{id}.kset"));
    if karakuri_store::fs::exists(&preset_path) {
        return read_preset_set(id, &preset_path, &presets_dir);
    }

    // 2. Check store
    if !gate.allows_store() {
        return Err(format!(
            "reading set `{id}` from store is not permitted while the Library gate is preset — \
             access to the operator's private store requires the Library gate set to 'all' \
             at the head of the Library bay"
        ));
    }

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
    // Preserves file declaration order of nodes as written.
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
        if let Ok(bytes) = store.get_artifact(&hash) {
            let src = String::from_utf8_lossy(&bytes);
            out.push_str("\n```kir\n");
            out.push_str(&src);
            if !src.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
    }
    let mut params = Vec::new();

    let mut bindings = Vec::new();
    for line in &lines {
        match line.record() {
            Record::Param { at, key, value } => {
                params.push((*at, key.clone(), *value));
            }
            Record::Bind {
                layer,
                index,
                key,
                signal,
                curve,
                range,
                ..
            } => {
                bindings.push((
                    *layer,
                    *index,
                    key.clone(),
                    signal.clone(),
                    curve.clone(),
                    *range,
                ));
            }
            _ => {}
        }
    }

    if !params.is_empty() {
        out.push_str("\nParameters:\n");
        for (at, key, val) in params {
            let at_str = match at {
                Some(addr) => format!("{}:{}", addr.layer, addr.index),
                None => "*".to_string(),
            };
            out.push_str(&format!("- `{at_str} {key}` = {val:?}\n"));
        }
    }

    if !bindings.is_empty() {
        out.push_str("\nBindings:\n");
        for (layer, idx, key, sig, curve, range) in bindings {
            let at = match idx {
                Some(i) => format!("{layer}:{i}"),
                None => format!("{layer}"),
            };
            out.push_str(&format!(
                "- `{at} {key}` <- `{sig}` ({curve}, [{:.2}, {:.2}])\n",
                range[0], range[1]
            ));
        }
    }

    out.push('\n');
    out.push_str(&element_storage_block(&store, id));
    Ok(out)
}

struct PresetBinding {
    layer: String,
    index: Option<u32>,
    key: String,
    signal: String,
    curve: String,
    range: [f64; 2],
}

fn read_preset_set(
    id: &str,
    path: &std::path::Path,
    presets_dir: &std::path::Path,
) -> Result<String, String> {
    let content =
        karakuri_store::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut parts: Vec<(String, u32, String)> = Vec::new();
    let mut edges: Vec<(String, String, String)> = Vec::new();
    let mut params: Vec<(String, Option<u32>, String, Value)> = Vec::new();
    let mut bindings: Vec<PresetBinding> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(rec) = serde_json::from_str::<Value>(line) {
            match rec.get("t").and_then(Value::as_str) {
                Some("part") => {
                    let layer = rec
                        .get("layer")
                        .and_then(Value::as_str)
                        .unwrap_or("L1")
                        .to_string();
                    let index = rec.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;
                    let p = rec
                        .get("path")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    parts.push((layer, index, p));
                }
                Some("edge") => {
                    let node = rec
                        .get("node")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let slot = rec
                        .get("slot")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let to = rec
                        .get("to")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    edges.push((node, slot, to));
                }
                Some("param") => {
                    let layer = rec
                        .get("layer")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let index = rec.get("index").and_then(Value::as_u64).map(|i| i as u32);
                    let key = rec
                        .get("key")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    if let Some(val) = rec.get("value") {
                        params.push((layer, index, key, val.clone()));
                    }
                }
                Some("bind") => {
                    let layer = rec
                        .get("layer")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let index = rec.get("index").and_then(Value::as_u64).map(|i| i as u32);
                    let key = rec
                        .get("key")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let sig = rec
                        .get("signal")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let curve = rec
                        .get("curve")
                        .and_then(Value::as_str)
                        .unwrap_or("lin")
                        .to_string();
                    let r = rec.get("range").and_then(Value::as_array);
                    let r0 = r
                        .and_then(|a| a.first())
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    let r1 = r
                        .and_then(|a| a.get(1))
                        .and_then(Value::as_f64)
                        .unwrap_or(1.0);
                    bindings.push(PresetBinding {
                        layer,
                        index,
                        key,
                        signal: sig,
                        curve,
                        range: [r0, r1],
                    });
                }
                _ => {}
            }
        }
    }

    let mut out = format!(
        "preset set `{id}` holds {} node{}, and `--load-set {id}` plays it. Everything below \
         is what each procedure declares.\n",
        parts.len(),
        if parts.len() == 1 { "" } else { "s" }
    );

    for (layer, index, file_name) in parts {
        let proc_path = presets_dir.join(&file_name);
        let src = karakuri_store::fs::read_to_string(&proc_path).unwrap_or_default();
        let stem = std::path::Path::new(&file_name)
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or(&file_name);
        out.push_str(&format!("\n{layer}:{index} `{stem}` ({file_name})\n"));
        if !src.is_empty() {
            out.push_str("```kir\n");
            out.push_str(&src);
            if !src.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
    }

    if !edges.is_empty() {
        out.push_str("\nEdges:\n");
        for (node, slot, to) in edges {
            out.push_str(&format!("- `{node}.{slot}` -> `{to}`\n"));
        }
    }

    if !params.is_empty() {
        out.push_str("\nParameters:\n");
        for (layer, idx, key, val) in params {
            let at = match idx {
                Some(i) => format!("{layer}:{i}"),
                None => layer,
            };
            out.push_str(&format!("- `{at} {key}` = {val}\n"));
        }
    }

    if !bindings.is_empty() {
        out.push_str("\nBindings:\n");
        for b in bindings {
            let at = match b.index {
                Some(i) => format!("{}:{i}", b.layer),
                None => b.layer,
            };
            out.push_str(&format!(
                "- `{at} {}` <- `{}` ({}, [{:.2}, {:.2}])\n",
                b.key, b.signal, b.curve, b.range[0], b.range[1]
            ));
        }
    }

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
    // An empty store returns a distinct informative message before filters run.
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
        // Notify caller of list truncation when results exceed LISTED limit.
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
    // Report unreadable set files explicitly rather than omitting them.
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
