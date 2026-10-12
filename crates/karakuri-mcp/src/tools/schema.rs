use std::sync::{Arc, Mutex};

use agent_man::mcp::{HandlerResult, ParamType, Tool, ToolBuilder, ToolResult};
use serde_json::{json, Value};

use super::called_to_handler_result;
use super::dispatch_tool;
use super::tool_result;
use crate::{doc_engine, layer_name, Slots, State, LAYERS};

fn tool_handler(
    name: &'static str,
    state: Arc<Mutex<State>>,
) -> impl Fn(Value) -> HandlerResult<ToolResult> + Send + Sync + 'static {
    move |args| {
        let mut state = match state.lock() {
            Ok(s) => s,
            Err(_) => return ToolResult::error("mcp state mutex is poisoned").into(),
        };
        let called = match dispatch_tool(name, &args, &mut state) {
            Ok(c) => c,
            Err(e) => return tool_result(Err(e)).into(),
        };
        drop(state);
        called_to_handler_result(called)
    }
}

/// Builds all 14 MCP tools with concise descriptions, documentation links, and argument validation.
pub(crate) fn build_tools(state: Arc<Mutex<State>>) -> Vec<Tool> {
    let layers: Vec<String> = LAYERS.iter().map(|l| layer_name(*l).to_string()).collect();
    let operable_titles: Vec<String> = crate::operable()
        .iter()
        .map(|row| row.title().to_string())
        .collect();

    vec![
        // 1. read_procedure
        ToolBuilder::new(
            "read_procedure",
            "Read the source of one node of a deck slot by layer and index. Read before writing to guide your edit.",
        )
        .param("slot", ParamType::Integer { min: Some(0), max: None }, "deck slot, from 0", true)
        .param("layer", ParamType::Enum(layers.clone()), "node layer (L1, L2, L3, L4, Field)", true)
        .param("index", ParamType::Integer { min: Some(0), max: None }, "node index on that layer, from 0", false)
        .see("ir-spec")
        .example(json!({ "slot": 0, "layer": "L1" }))
        .handler(tool_handler("read_procedure", state.clone()))
        .build(),

        // 2. write_procedure
        ToolBuilder::new(
            "write_procedure",
            "Check and write a procedure for a deck slot. The check is of this procedure alone; slot rebuild and swap errors are reported by swap_outcome. A `uses` declaration needs an `edge`, written by wire_input.",
        )
        .param("slot", ParamType::Integer { min: Some(0), max: None }, "deck slot, from 0", true)
        .param("layer", ParamType::Enum(layers.clone()), "node layer (L1, L2, L3, L4, Field)", true)
        .param("index", ParamType::Integer { min: Some(0), max: None }, "node index on that layer, from 0", false)
        .param("source", ParamType::String, "the whole procedure source", true)
        .see("swap-lifecycle")
        .example(json!({ "slot": 0, "layer": "L1", "source": "kind L1\nvoid main() {}" }))
        .handler(tool_handler("write_procedure", state.clone()))
        .build(),

        // 3. wire_input
        ToolBuilder::new(
            "wire_input",
            "Bind one node's declared input to another node in the same deck slot. An edge already binding this input is replaced. Nothing here unbinds one. Build errors are reported by swap_outcome. Both ends are node names, not addresses.",
        )
        .param("slot", ParamType::Integer { min: Some(0), max: None }, "deck slot, from 0: whose Set this edge is about", true)
        .param("node", ParamType::String, "the node that declares the input, by name", true)
        .param("input", ParamType::String, "what that node's procedure calls the input", true)
        .param("to", ParamType::String, "the node bound to it, by name", true)
        .see("edges-and-wiring")
        .example(json!({ "slot": 0, "node": "morph", "input": "far", "to": "sphere_shell" }))
        .handler(tool_handler("wire_input", state.clone()))
        .build(),

        // 4. swap_outcome
        ToolBuilder::new(
            "swap_outcome",
            "Query recent swap and rebuild status for deck slots: whether a written procedure landed, was overloaded, or failed to build.",
        )
        .see("swap-lifecycle")
        .example(json!({}))
        .handler(tool_handler("swap_outcome", state.clone()))
        .build(),

        // 5. save_set
        ToolBuilder::new(
            "save_set",
            "Keep what a slot is playing as a Set file in `<store>/sandbox/`. Saves live state: parameter values, capacit(ies), binding(s), seed(s), edge(s), composite/live renderer, and camera.",
        )
        .param("slot", ParamType::Integer { min: Some(0), max: None }, "deck slot, from 0", true)
        .param("id", ParamType::String, "optional target filename ID", false)
        .see("set-storage")
        .example(json!({ "slot": 0 }))
        .handler(tool_handler("save_set", state.clone()))
        .build(),

        // 6. read_set
        ToolBuilder::new(
            "read_set",
            "Read declarations and node structures of a saved Set in the library without compiling or activating it.",
        )
        .param("id", ParamType::String, "the id the set was filed under", true)
        .see("set-storage")
        .example(json!({ "id": "default" }))
        .handler(tool_handler("read_set", state.clone()))
        .build(),

        // 7. list_sets
        ToolBuilder::new(
            "list_sets",
            "List saved Sets in the library with node addresses and names, most recently written first.",
        )
        .param("holds", ParamType::String, "list only sets holding a node whose name contains this", false)
        .param("layer", ParamType::Enum(layers.clone()), "list only sets holding a node on this layer", false)
        .see("set-storage")
        .example(json!({}))
        .handler(tool_handler("list_sets", state.clone()))
        .build(),

        // 8. walk_history
        ToolBuilder::new(
            "walk_history",
            "List all compiled versions of a Set filed under `<store>/history/`, most recent first.",
        )
        .param("set", ParamType::String, "the Set whose versions to walk, by id", true)
        .see("set-storage")
        .example(json!({ "set": "default" }))
        .handler(tool_handler("walk_history", state.clone()))
        .build(),

        // 9. operate
        ToolBuilder::new(
            "operate",
            "Execute an instrument operation by heading name from the operations vocabulary. Audited against current console openings.",
        )
        .param("operation", ParamType::Enum(operable_titles), "heading of the operation to perform", true)
        .skip_validation(true)
        .see("operations")
        .example(json!({ "operation": "Choose which renderer of a deck is live" }))
        .handler(tool_handler("operate", state.clone()))
        .build(),

        // 10. get_permissions
        ToolBuilder::new(
            "get_permissions",
            "Discover current console constraints: bay openings, per-slot MCP policies, mixer status, and writability.",
        )
        .see("operations")
        .example(json!({}))
        .handler(tool_handler("get_permissions", state.clone()))
        .build(),

        // 11. read_slot
        ToolBuilder::new(
            "read_slot",
            "Read all procedural nodes (L1, L2, L3, L4, Field) for a deck slot in a single call.",
        )
        .param("slot", ParamType::Integer { min: Some(0), max: None }, "deck slot, from 0", true)
        .see("set-storage")
        .example(json!({ "slot": 0 }))
        .handler(tool_handler("read_slot", state.clone()))
        .build(),

        // 12. copy_slot
        ToolBuilder::new(
            "copy_slot",
            "Atomically copy procedural nodes from one deck slot to another, with optional layer filtering.",
        )
        .param("from_slot", ParamType::Integer { min: Some(0), max: None }, "source deck slot, from 0", true)
        .param("to_slot", ParamType::Integer { min: Some(0), max: None }, "destination deck slot, from 0", true)
        .param("layer", ParamType::Enum(layers), "optional layer filter", false)
        .see("set-storage")
        .example(json!({ "from_slot": 0, "to_slot": 1 }))
        .handler(tool_handler("copy_slot", state.clone()))
        .build(),

        // 13. check_procedure
        ToolBuilder::new(
            "check_procedure",
            "Statically validate .kir shader syntax, types, and resource costs without modifying GPU or memory state.",
        )
        .param("source", ParamType::String, "the .kir shader source code to validate", true)
        .see("ir-spec")
        .example(json!({ "source": "kind L1\nvoid main() {}" }))
        .handler(tool_handler("check_procedure", state.clone()))
        .build(),

        // 14. check_set
        ToolBuilder::new(
            "check_set",
            "Validate Set configuration, knob mappings, and asset integrity in the store.",
        )
        .param("id", ParamType::String, "the Set ID to validate", true)
        .see("set-storage")
        .example(json!({ "id": "default" }))
        .handler(tool_handler("check_set", state))
        .build(),
    ]
}

/// Returns all registered MCP tool definitions as a JSON array (for tests and backwards compatibility).
pub fn tools() -> Value {
    let dummy_slots = Slots::unpointed();
    let (_tx, rx) = std::sync::mpsc::sync_channel(1);
    let (asked, _requests) = std::sync::mpsc::sync_channel(1);
    let (wiring, _wires) = std::sync::mpsc::sync_channel(1);
    let (operating, _operations) = std::sync::mpsc::sync_channel(1);
    let state = Arc::new(Mutex::new(State {
        slots: dummy_slots,
        store: std::path::PathBuf::from("/tmp"),
        watching: false,
        opening: karakuri_environment::Opening::closed(),
        slot_policies: karakuri_environment::SlotPolicies::new(),
        events: rx,
        asked,
        wiring,
        operating,
        recent: Vec::new(),
        dropped: Arc::new(std::sync::atomic::AtomicU64::new(0)),
    }));
    let mut engine = doc_engine();
    for tool in build_tools(state) {
        let _ = engine.register_tool(tool);
    }
    serde_json::to_value(engine.mcp_list_tools()).unwrap_or_default()
}
