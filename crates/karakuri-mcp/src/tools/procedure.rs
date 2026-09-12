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
    }
}

/// `write_procedure`'s arguments as the operation they name.
///
/// `source` is parsed before the slot is checked, which is the order this tool
/// has always refused in: a call with no `source` at all is told that first,
/// whatever slot it named.
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

/// [`Operation::ReadProcedure`], done: the source of one node of one deck.
///
/// The address arrives as the vocabulary's [`NodeAddress`] and is turned back
/// into the compiler's own [`Kind`] here, at the one place that resolves a file
/// — see [`kind_of`].
pub(crate) fn read_procedure(deck: u8, node: NodeAddress, state: &State) -> Result<String, String> {
    let path = state
        .slots
        .path(usize::from(deck), kind_of(node.layer), node.index as usize)?;
    std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
}

/// [`Operation::WriteProcedure`], done: check a procedure and, if it compiles,
/// write it.
///
/// The record this owes is `Record::Procedure` and it is not written here —
/// `karakuri_operation_record::written` answers `Silent(OnLanding)`, because a
/// record written at the ask would claim a swap the budget went on to roll
/// back. It is written where the swap lands, which is the render loop, and this
/// tool's answer says as much.
pub(crate) fn write_procedure(
    deck: u8,
    node: NodeAddress,
    source: &str,
    state: &State,
) -> Result<String, String> {
    let slot = usize::from(deck);
    let layer = kind_of(node.layer);
    let index = node.index as usize;
    let path = state.slots.path(slot, layer, index)?;
    let name = layer_name(layer);

    // **Checked before it is written, and the diagnostics are handed back.**
    // Writing first and letting the watcher report would put the compiler's
    // answer on a terminal the model cannot see.
    let report = check_procedure(source);
    if !report.success {
        return Err(serde_json::to_string_pretty(&report).unwrap_or_default());
    }
    let checked = karakuri_environment::compile::check(source).map_err(|e| {
        let fallback = DiagnosticReport {
            diagnostics: vec![Diagnostic {
                code: "KIR-E100-COMPILE-FAILED".to_string(),
                message: format!("compile: {e}"),
                line: None,
                column: None,
                remedy: None,
            }],
            success: false,
        };
        serde_json::to_string_pretty(&fallback).unwrap_or(e)
    })?;
    // **The address and the source have to agree**, and the comparison is now
    // between two `Kind`s rather than between a string and a guess. The guess
    // was `L1`, or `L4` for everything else, which made this refusal answer
    // about a layer nobody had named: a `kind L2` sent to a slot's L2 was
    // turned away for not being a renderer, which is a refusal about a mistake
    // the caller had not made.
    if checked.kind != layer {
        let report = DiagnosticReport {
            diagnostics: vec![Diagnostic {
                code: "KIR-E300-LAYER-MISMATCH".to_string(),
                message: format!(
                    "this is a {:?} procedure and it was addressed to slot {slot}'s {name} — \
                     the two layers are not interchangeable, and what a file is is the `kind` \
                     line inside it",
                    checked.kind
                ),
                line: None,
                column: None,
                remedy: Some(format!(
                    "Change `kind {:?}` to match `{name}` or address the appropriate slot layer.",
                    checked.kind
                )),
            }],
            success: false,
        };
        return Err(serde_json::to_string_pretty(&report).unwrap_or_default());
    }

    // **What else this write reaches**, which is normally nothing now and is
    // still asked. `scratch::materialise` gives every slot its own copy, so two
    // slots of a run cannot hold one path however the operator spelled the
    // command line, and this scan comes back empty *because of that rule*
    // rather than because nobody happens to share. It is kept, and kept as a
    // scan rather than replaced by the constant it usually equals: `Slots` is a
    // list of paths this module is handed, the sentence is true of whatever it
    // is handed, and a `Vec::new()` written here would be this module asserting
    // something about its caller.
    //
    // It was load-bearing under the rule that went: `watch.rs` documents two
    // slots sharing a pair as supported, and the manual's own example gave one
    // `soft_points.kir` to three slots — so naming one slot was reporting a
    // third of what happened. Anything skipped or widened is said with a count.
    //
    // **Every node of every other slot**, whatever layer it is on: the scan
    // walked an L1 and a list of renderers, which is the shape a slot had
    // before it could hold a deformation chain — so one `swirl_warp.kir` given
    // to two slots was a write that silently changed both and named one.
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

    std::fs::write(&path, source).map_err(|e| format!("{}: {e}", path.display()))?;
    let shared = if also.is_empty() {
        String::new()
    } else {
        format!(
            " This file is also slot{} {}, which now show the same procedure.",
            if also.len() == 1 { "" } else { "s" },
            also.join(", ")
        )
    };
    // Named as an address rather than as a layer, because two renderers or two
    // sources are only told apart by the index — the same `layer:index:`
    // `--param` writes.
    // **The edit history is true of both branches**, and it used to be said in
    // only one. `--mcp` on its own makes a run editable exactly as `--watch`
    // does — `main.rs`'s `editable` is `watch || mcp.is_some()` — so the same
    // two things have already happened either way: the deck runs from copies in
    // the scratch, so the paths the operator named on the command line are not
    // what this wrote to, and `history::seed` has filed the version the run
    // started with. "It replaced the file on disk and there is no backup" was
    // wrong about both halves, on the one surface whose reader has no other way
    // to find out.
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
