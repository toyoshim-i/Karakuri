use std::path::PathBuf;

use karakuri_environment::{Opening, SlotPolicies};
use karakuri_operation::gate::{Class, Open};
use serde_json::json;

use super::*;
use crate::in_process::in_process;

#[test]
fn in_process_lists_tools_and_handles_direct_calls() {
    let opening = Opening::closed();
    opening.set(Open::CLOSED.with(Class::MixFaders, true));
    let slot_policies = SlotPolicies::default();
    let slots = slots();
    let store = PathBuf::from("a/store");

    let (reporter, mcp) = in_process(slots, store, true, opening, slot_policies);

    // Verify tools definition
    let tools = mcp.tools();
    assert!(tools.is_array());
    let list = tools.as_array().expect("array of tools");
    assert!(list
        .iter()
        .any(|t| t.get("name") == Some(&json!("operate"))));
    assert!(list
        .iter()
        .any(|t| t.get("name") == Some(&json!("read_procedure"))));

    // Non-blocking call to an immediate tool (e.g. invalid/refused or inspection)
    let mut pending = mcp
        .call_tool("list_sets", json!({}))
        .expect("dispatch list_sets");
    let result = pending.poll_settled().expect("poll list_sets");
    assert!(result.is_some(), "list_sets completes synchronously");

    // Call an operation that requires render loop execution (Gain under MixFaders)
    let mut pending_op = mcp
        .call_tool(
            "operate",
            json!({
                "operation": "Gain",
                "with": {
                    "deck": 0,
                    "gain": 0.5,
                }
            }),
        )
        .expect("dispatch Gain");

    // Initially not settled because the render loop hasn't drained it
    let polled = pending_op.poll_settled().unwrap();
    assert!(
        polled.is_none(),
        "operate should await the render loop but got: {polled:?}"
    );

    // The render loop drains the request
    let operations: Vec<_> = reporter.operations().collect();
    assert_eq!(operations.len(), 1);
    let op = &operations[0];
    op.reply.clone().settled(Ok("faded".into()));

    // Now polling should succeed with the settled answer
    let finished = pending_op.poll_settled().unwrap();
    assert!(finished.is_some());
    let val = finished.unwrap();
    assert_eq!(val["result"]["content"][0]["text"], "faded");
}
