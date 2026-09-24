use std::sync::mpsc;

use karakuri_operation::Operation;
use serde_json::Value;

use super::super::*;
use super::*;

/// Parses `wire_input` arguments into an `Operation::WireInput`.
pub(crate) fn wired_input(args: &Value, slots: &Slots) -> Result<Operation, String> {
    let slot = args
        .get("slot")
        .and_then(Value::as_u64)
        .ok_or("`slot` is required and is a number")? as usize;
    let node = edge_name(args, "node", "the node that declares the input")?;
    let input = edge_name(args, "input", "what that node's procedure calls the input")?;
    let to = edge_name(args, "to", "the node bound to it")?;
    let deck = deck_named(slot, slots)?;
    Ok(Operation::WireInput {
        deck,
        node,
        // Maps the MCP schema `input` argument to `Operation::WireInput::slot`.
        slot: input.into(),
        to,
    })
}

/// Extracts a non-empty string argument for an edge component from `args`.
pub(crate) fn edge_name(args: &Value, key: &str, what: &str) -> Result<String, String> {
    let named = args
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("`{key}` is required and is a string: {what}"))?;
    if named.is_empty() {
        return Err(format!(
            "`{key}` is empty, and every part of an edge names something: a node, the \
             input it declares, and the node bound to it"
        ));
    }
    Ok(named.to_string())
}

/// Dispatches a wire input request to the render loop.
pub(crate) fn wire_input(
    deck: u8,
    node: &str,
    input: &str,
    to: &str,
    state: &State,
) -> Result<(mpsc::Receiver<News>, String), String> {
    let slot = usize::from(deck);
    state
        .slot_policies
        .check_writable_detail(slot)
        .map_err(|d| refusal_payload(&d))?;
    let (tx, rx) = mpsc::channel();
    state
        .wiring
        .try_send(WireRequest {
            slot,
            edge: karakuri_engine::set::Edge {
                node: node.to_string(),
                slot: input.into(),
                to: to.to_string(),
            },
            reply: Reply(tx),
        })
        // **Answered rather than waited for**, in [`save_set`]'s two shapes and
        // its words: a queue nobody is emptying and a loop that has ended are
        // different facts, and neither of them may leave a model holding a call.
        .map_err(|e| match e {
            mpsc::TrySendError::Full(_) => format!(
                "the render loop has {ASKED} edges queued and no room for another: it is \
                 taking them slower than they are arriving, or it is not running frames at \
                 all. Nothing was rewired, and asking again is safe"
            ),
            mpsc::TrySendError::Disconnected(_) => {
                "the render loop has ended: this run is shutting down and nothing was \
                 rewired"
                    .to_string()
            }
        })?;
    // **What the loop's own sentence will not say.** The loop knows what it did
    // with the edge; only this side knows how the run was started, and a run
    // without `--watch` has no watcher to rebuild the slot with the new wiring —
    // which is the same thing `write_procedure` says about a file nothing will
    // pick up, about the other half of one edit.
    let note = if state.watching {
        String::new()
    } else {
        "**This run was started without `--watch`, so no watcher will rebuild the slot** \
         — what is on screen was built with the wiring this run started with and will go \
         on being it. The edge is the run's from here on, so a `save_set` of this slot \
         records it; the picture does not change."
            .to_string()
    };
    Ok((rx, note))
}
