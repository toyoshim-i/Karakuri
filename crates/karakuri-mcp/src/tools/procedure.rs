use karakuri_ir::{Diagnostic, DiagnosticReport};
use karakuri_operation::{NodeAddress, Operation};
use serde_json::Value;

use super::super::*;
use super::*;

/// Check a procedure source against the full IR pipeline (parse, type,
/// contract, cost) and return a machine-readable `DiagnosticReport`.
pub fn check_procedure(source: &str) -> DiagnosticReport {
    let mut diagnostics = Vec::new();
    match karakuri_ir::parse(source) {
        Err(errs) => {
            diagnostics.extend(errs.iter().map(|e| Diagnostic::from_ir_error(e, source)));
        }
        Ok(proc) => match karakuri_ir::check::check(&proc) {
            Err(errs) => {
                diagnostics.extend(errs.iter().map(|e| Diagnostic::from_ir_error(e, source)));
            }
            Ok(checked) => {
                if let Err(errs) = karakuri_ir::cost::estimate(&checked) {
                    diagnostics.extend(errs.iter().map(|e| Diagnostic::from_ir_error(e, source)));
                }
            }
        },
    }
    let success = diagnostics.is_empty();
    DiagnosticReport {
        diagnostics,
        success,
        hint: None,
    }
}

/// Parses `write_procedure` arguments into an `Operation::WriteProcedure`.
pub(crate) fn written_procedure(args: &Value, slots: &Slots) -> Result<Operation, String> {
    let (slot, layer, index) = slot_layer_index(args)?;
    let source = args
        .get("source")
        .and_then(Value::as_str)
        .ok_or("`source` is required")?;
    let deck = deck_named(slot, slots)?;
    Ok(Operation::WriteProcedure {
        deck,
        node: NodeAddress {
            layer: layer_of(layer),
            index: index as u32,
        },
        source: source.to_string(),
    })
}

/// Reads the source text of a procedure at the specified deck and node address.
pub(crate) fn read_procedure(deck: u8, node: NodeAddress, state: &State) -> Result<String, String> {
    let path = state
        .slots
        .path(usize::from(deck), kind_of(node.layer), node.index as usize)?;
    karakuri_store::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Reads a procedure's source text by name across active deck slots, store, or built-in presets.
pub(crate) fn read_procedure_by_name(name: &str, state: &State) -> Result<String, String> {
    let stem = name.strip_suffix(".kir").unwrap_or(name);

    // 1. Check active slots
    let slot_count = state.slots.count();
    for slot in 0..slot_count {
        if let Ok(nodes) = state.slots.nodes(slot) {
            for (_, _, path) in nodes {
                if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if file_stem == stem {
                        if let Ok(content) = karakuri_store::fs::read_to_string(&path) {
                            return Ok(content);
                        }
                    }
                }
            }
        }
    }

    // 2. Check store procedures
    if let Ok(store) = karakuri_store::Store::open(&state.store) {
        if let Ok(bytes) = store.read_procedure(stem) {
            return Ok(String::from_utf8_lossy(&bytes).to_string());
        }
    }

    // 3. Check presets
    let presets_dir = super::library::resolve_presets_dir();
    let preset_file = presets_dir.join(format!("{stem}.kir"));
    if karakuri_store::fs::exists(&preset_file) {
        if let Ok(content) = karakuri_store::fs::read_to_string(&preset_file) {
            return Ok(content);
        }
    }

    Err(format!(
        "no procedure `{name}` found in active slots, store at `{}`, or presets at `{}`",
        state.store.display(),
        presets_dir.display()
    ))
}

/// Validates and atomically writes a new procedure source to the slot node's backing file.
pub(crate) fn write_procedure(
    deck: u8,
    node: NodeAddress,
    source: &str,
    state: &State,
) -> Result<String, String> {
    let slot = usize::from(deck);
    state
        .slot_policies
        .check_writable_detail(slot)
        .map_err(|d| refusal_payload(&d))?;
    let layer = kind_of(node.layer);
    let index = node.index as usize;
    let path = state.slots.path(slot, layer, index)?;
    let name = layer_name(layer);

    // Validate syntax and compilation before writing to disk.
    let report = check_procedure(source);
    if !report.success {
        return Err(serde_json::to_string_pretty(&report).unwrap_or_default());
    }
    let checked = karakuri_environment::compile::check(source).map_err(|e| {
        let fallback = DiagnosticReport {
            diagnostics: vec![Diagnostic::new(
                karakuri_ir::error::codes::E100_COMPILE_FAILED,
                format!("compile: {e}"),
                None,
                None,
                None,
            )],
            success: false,
            hint: None,
        };
        serde_json::to_string_pretty(&fallback).unwrap_or(e)
    })?;
    // Ensure the declared procedure kind matches the target layer.
    if checked.kind != layer {
        let report = DiagnosticReport {
            diagnostics: vec![Diagnostic::new(
                karakuri_ir::error::codes::E300_LAYER_MISMATCH,
                format!(
                    "this is a {:?} procedure and it was addressed to slot {slot}'s {name} — \
                     the two layers are not interchangeable, and what a file is is the `kind` \
                     line inside it",
                    checked.kind
                ),
                None,
                None,
                Some(format!(
                    "Change `kind {:?}` to match `{name}` or address the appropriate slot layer.",
                    checked.kind
                )),
            )],
            success: false,
            hint: None,
        };
        return Err(serde_json::to_string_pretty(&report).unwrap_or_default());
    }

    // Identify any other slots sharing this backing path.
    let also: Vec<String> = (0..state.slots.count())
        .filter(|other| *other != slot)
        .filter(|other| {
            state
                .slots
                .nodes(*other)
                .is_ok_and(|nodes| nodes.iter().any(|(_, _, held)| *held == path))
        })
        .map(|other| other.to_string())
        .collect();

    karakuri_environment::scratch::write_atomic(&path, source.as_bytes())?;
    let shared = if also.is_empty() {
        String::new()
    } else {
        format!(
            " This file is also slot{} {}, which now show the same procedure.",
            if also.len() == 1 { "" } else { "s" },
            also.join(", ")
        )
    };
    // Inform the caller about the scratch replacement and edit history.
    let kept = "This replaced the run's copy of the file, under the scratch the deck runs \
                from — the paths named on the command line are not written to. The version \
                it replaced is in the run's edit history under `<store>/history/`, so it can \
                be got back, but not from here.";
    Ok(if state.watching {
        format!(
            "compiled and written to slot {slot} {name}:{index}.{shared} It is being built on a \
             worker thread and will swap in at a frame boundary; call `swap_outcome` to \
             find out whether it landed or was overloaded.\n\n{kept} Every later \
             version that compiles is kept there too."
        )
    } else {
        format!(
            "compiled and written to slot {slot} {name}:{index}.{shared} **This run was started \
             without `--watch`, so nothing will pick it up** — the file has changed and the \
             screen has not.\n\n{kept} Only the version the run started with is there: \
             nothing is snapshotting without `--watch`, so writing this node twice \
             replaces the first write with no record of it."
        )
    })
}
