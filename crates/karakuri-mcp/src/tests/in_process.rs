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
    let res = mcp.call_tool("list_sets", json!({}));
    assert!(matches!(res, agent_man::mcp::HandlerResult::Ready(_)));

    // Call an operation that requires render loop execution (Gain under MixFaders)
    let res_op = mcp.call_tool(
        "operate",
        json!({
            "operation": "Gain",
            "with": {
                "deck": 0,
                "gain": 0.5,
            }
        }),
    );

    let mut pending_op = match res_op {
        agent_man::mcp::HandlerResult::Pending(p) => p,
        _ => panic!("expected Pending"),
    };

    // Initially not settled because the render loop hasn't drained it
    let polled = pending_op.poll_settled();
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
    let finished = pending_op.poll_settled();
    assert!(finished.is_some());
    let val = finished.unwrap();
    assert_eq!(val.content[0].as_text(), Some("faded"));
}

#[test]
fn in_process_capture_frame_handles_requests_and_returns_image() {
    let opening = Opening::closed();
    let slot_policies = SlotPolicies::default();
    let slots = slots();
    let store = PathBuf::from("a/store");

    let (reporter, mcp) = in_process(slots, store, true, opening, slot_policies);

    // Call capture_frame
    let res = mcp.call_tool(
        "capture_frame",
        json!({
            "target": "program",
            "max_dimension": 512,
        }),
    );

    let mut pending = match res {
        agent_man::mcp::HandlerResult::Pending(p) => p,
        _ => panic!("expected Pending"),
    };

    assert!(pending.poll_settled().is_none());

    // The render loop drains the capture request
    let captures: Vec<_> = reporter.captures().collect();
    assert_eq!(captures.len(), 1);
    let req = &captures[0];
    assert_eq!(req.target, CaptureTarget::Program);
    assert_eq!(req.max_dimension, 512);

    // Reply with a mock captured image
    req.reply.clone().settled(Ok(CapturedImage {
        description: "captured program frame (512x288)".into(),
        data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGP6zwAAAgcBApocMXEAAAAASUVORK5CYII=".into(),
        mime_type: "image/png".into(),
        width: 512,
        height: 288,
    }));

    let finished = pending.poll_settled();
    assert!(finished.is_some());
    let tool_res = finished.unwrap();
    assert_eq!(tool_res.content.len(), 2);
    assert_eq!(
        tool_res.content[0].as_text(),
        Some("captured program frame (512x288)")
    );
    assert!(matches!(
        tool_res.content[1],
        agent_man::mcp::ToolContent::Image { .. }
    ));
}
