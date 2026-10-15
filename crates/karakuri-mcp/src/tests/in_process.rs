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

#[test]
fn in_process_write_procedure_batch_and_dynamic_node_addition() {
    let dir = tempfile::tempdir().expect("tempdir");
    let l1_path = dir.path().join("l1.kir");
    let l4_path = dir.path().join("l4.kir");

    let probe_l1 = "proc p_l1 {\nkind L1\ntopology points\ncapacity [1, 64] = 8\nemit position\nelement {\nposition = vec3(0.0);\n}\n}\n";
    let probe_l4 = "proc p_l4 {\nkind L4\nblend additive\nconsumes position\nvertex {\nclip = camera * vec4(position, 1.0);\npoint_rate = 0.008;\n}\nfragment {\ncolor = vec4(1.0);\n}\n}\n";
    let probe_l2 = "proc p_l2 {\nkind L2\nconsumes position\ndeform {\nposition = vec3(position.x, position.y * 1.5, position.z);\n}\n}\n";

    std::fs::write(&l1_path, probe_l1).expect("write l1");
    std::fs::write(&l4_path, probe_l4).expect("write l4");

    let slots = Slots::of(vec![(l1_path.clone(), vec![l4_path.clone()])]);
    let (reporter, mcp) = in_process(
        slots,
        dir.path().join("store"),
        true,
        Opening::closed(),
        SlotPolicies::default(),
    );

    // 1. Dynamic layer allocation: write L2 which is unallocated in slot 0.
    let res_dyn = mcp.call_tool(
        "write_procedure",
        json!({
            "slot": 0,
            "layer": "L2",
            "source": probe_l2,
        }),
    );
    let ready_dyn = match res_dyn {
        agent_man::mcp::HandlerResult::Ready(r) => r,
        _ => panic!("expected Ready"),
    };
    assert!(!ready_dyn.is_error);
    assert!(
        ready_dyn.content[0]
            .as_text()
            .unwrap()
            .contains("compiled and written"),
        "dynamic write should succeed"
    );

    // Read back L2 to verify it was allocated and written.
    let res_read = mcp.call_tool(
        "read_procedure",
        json!({
            "slot": 0,
            "layer": "L2",
        }),
    );
    let read_val = match res_read {
        agent_man::mcp::HandlerResult::Ready(r) => r,
        _ => panic!("expected Ready"),
    };
    assert!(!read_val.is_error);
    assert!(read_val.content[0].as_text().unwrap().contains("proc p_l2"));

    // Check that WriteProcedure operation was queued for the render loop
    let ops: Vec<_> = reporter.operations().collect();
    assert_eq!(ops.len(), 1);
    match &ops[0].operation {
        karakuri_operation::Operation::WriteProcedure { deck, node, source } => {
            assert_eq!(usize::from(*deck), 0);
            assert_eq!(node.layer, karakuri_operation::Layer::L2);
            assert!(source.contains("proc p_l2"));
        }
        other => panic!("unexpected operation: {other:?}"),
    }

    // 2. Batch update: update both L1 and L4 atomically.
    let updated_l1 = "proc p_l1_v2 {\nkind L1\ntopology points\ncapacity [1, 64] = 16\nemit position\nelement {\nposition = vec3(1.0);\n}\n}\n";
    let updated_l4 = "proc p_l4_v2 {\nkind L4\nblend additive\nconsumes position\nvertex {\nclip = camera * vec4(position, 1.0);\npoint_rate = 0.016;\n}\nfragment {\ncolor = vec4(0.5, 0.5, 0.5, 1.0);\n}\n}\n";

    let res_batch = mcp.call_tool(
        "write_procedure",
        json!({
            "slot": 0,
            "nodes": [
                { "layer": "L1", "source": updated_l1 },
                { "layer": "L4", "source": updated_l4 }
            ]
        }),
    );
    let ready_batch = match res_batch {
        agent_man::mcp::HandlerResult::Ready(r) => r,
        _ => panic!("expected Ready"),
    };
    assert!(
        !ready_batch.is_error,
        "batch write failed: {:?}",
        ready_batch.content[0].as_text()
    );
    assert!(
        ready_batch.content[0]
            .as_text()
            .unwrap()
            .contains("compiled and written 2 nodes atomically"),
        "batch write should succeed: {:?}",
        ready_batch.content[0].as_text()
    );

    // Verify both L1 and L4 read back the new sources.
    let res_read_l1 = mcp.call_tool("read_procedure", json!({ "slot": 0, "layer": "L1" }));
    let res_read_l4 = mcp.call_tool("read_procedure", json!({ "slot": 0, "layer": "L4" }));
    let text_l1 = match res_read_l1 {
        agent_man::mcp::HandlerResult::Ready(r) => r.content[0].as_text().unwrap().to_string(),
        _ => panic!("expected Ready"),
    };
    let text_l4 = match res_read_l4 {
        agent_man::mcp::HandlerResult::Ready(r) => r.content[0].as_text().unwrap().to_string(),
        _ => panic!("expected Ready"),
    };
    assert!(text_l1.contains("p_l1_v2"));
    assert!(text_l4.contains("p_l4_v2"));

    // 3. Batch atomic failure: if one node has invalid syntax, none should be updated.
    let failed_batch = mcp.call_tool(
        "write_procedure",
        json!({
            "slot": 0,
            "nodes": [
                { "layer": "L1", "source": "proc p_l1_v3 {\nkind L1\nemit position\n}" },
                { "layer": "L4", "source": "invalid syntax here" }
            ]
        }),
    );
    let ready_failed = match failed_batch {
        agent_man::mcp::HandlerResult::Ready(r) => r,
        _ => panic!("expected Ready"),
    };
    assert!(
        ready_failed.is_error,
        "batch write should fail on invalid syntax"
    );

    // Verify L1 was NOT updated to v3
    let res_read_l1_after = mcp.call_tool("read_procedure", json!({ "slot": 0, "layer": "L1" }));
    let text_l1_after = match res_read_l1_after {
        agent_man::mcp::HandlerResult::Ready(r) => r.content[0].as_text().unwrap().to_string(),
        _ => panic!("expected Ready"),
    };
    assert!(
        text_l1_after.contains("p_l1_v2"),
        "L1 should remain at v2 due to atomic rollback"
    );
}

#[test]
fn in_process_load_procedure_on_offline_and_writable_deck() {
    let slots = slots();
    let slot_policies = SlotPolicies::default();
    // slot 1 is offline (in_mix = false by default)
    let opening = Opening::closed();
    let (reporter, mcp) = in_process(
        slots,
        PathBuf::from("a/store"),
        true,
        opening,
        slot_policies,
    );

    // Call LoadProcedure on offline deck 1 with Program bay closed: should be allowed (Standing::Open)
    let res = mcp.call_tool(
        "operate",
        json!({
            "operation": "Load a procedure over a layer",
            "with": {
                "deck": 1,
                "procedure": "orbit_wide"
            }
        }),
    );
    let mut pending = match res {
        agent_man::mcp::HandlerResult::Pending(p) => p,
        agent_man::mcp::HandlerResult::Ready(r) => {
            panic!("expected Pending, got error: {:?}", r.content[0].as_text())
        }
    };
    let ops: Vec<_> = reporter.operations().collect();
    assert_eq!(ops.len(), 1);
    ops[0].reply.clone().settled(Ok("loaded".into()));
    let finished = pending.poll_settled().unwrap();
    assert_eq!(finished.content[0].as_text(), Some("loaded"));
}

#[test]
fn in_process_add_chain_effect_accepts_preset_name_and_content_address() {
    let slots = slots();
    let slot_policies = SlotPolicies::default();
    let opening = Opening::closed();
    opening.set(Open::CLOSED.with(Class::MasterEffects, true));
    let (reporter, mcp) = in_process(
        slots,
        PathBuf::from("a/store"),
        true,
        opening,
        slot_policies,
    );

    // 1. Calling with preset name "bloom"
    let res_name = mcp.call_tool(
        "operate",
        json!({
            "operation": "Add an effect to the master chain",
            "with": {
                "procedure": "bloom"
            }
        }),
    );
    let mut pending_name = match res_name {
        agent_man::mcp::HandlerResult::Pending(p) => p,
        agent_man::mcp::HandlerResult::Ready(r) => {
            panic!("expected Pending, got: {:?}", r.content[0].as_text())
        }
    };
    let ops: Vec<_> = reporter.operations().collect();
    assert_eq!(ops.len(), 1);
    match &ops[0].operation {
        Operation::AddChainEffect { procedure, cut } => {
            assert_eq!(procedure, "bloom");
            assert_eq!(*cut, None);
        }
        other => panic!("expected AddChainEffect, got {other:?}"),
    }
    ops[0].reply.clone().settled(Ok("added bloom".into()));
    let finished_name = pending_name.poll_settled().unwrap();
    assert_eq!(finished_name.content[0].as_text(), Some("added bloom"));

    // 2. Calling with sha256 content address
    let sha = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let res_sha = mcp.call_tool(
        "operate",
        json!({
            "operation": "Add an effect to the master chain",
            "with": {
                "procedure": sha,
                "cut": "mix"
            }
        }),
    );
    let mut pending_sha = match res_sha {
        agent_man::mcp::HandlerResult::Pending(p) => p,
        agent_man::mcp::HandlerResult::Ready(r) => {
            panic!("expected Pending, got: {:?}", r.content[0].as_text())
        }
    };
    let ops2: Vec<_> = reporter.operations().collect();
    assert_eq!(ops2.len(), 1);
    match &ops2[0].operation {
        Operation::AddChainEffect { procedure, cut } => {
            assert_eq!(procedure, sha);
            assert_eq!(*cut, Some(karakuri_operation::Cut::Mix));
        }
        other => panic!("expected AddChainEffect, got {other:?}"),
    }
    ops2[0].reply.clone().settled(Ok("added sha".into()));
    let finished_sha = pending_sha.poll_settled().unwrap();
    assert_eq!(finished_sha.content[0].as_text(), Some("added sha"));
}
