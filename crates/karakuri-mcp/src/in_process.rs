//! In-process MCP dispatching for WebAssembly, tests, and in-memory agent harnesses.

use std::sync::{mpsc, Arc, Mutex};

use agent_man::mcp::{HandlerResult, ToolResult};
use agent_man::DocEngine;
use serde_json::{json, Value};

use crate::resources::doc_engine_with_state;
use crate::*;

/// In-process handle to the Karakuri MCP engine.
#[derive(Clone)]
pub struct InProcessMcp {
    pub(crate) engine: Arc<DocEngine>,
    #[allow(dead_code)]
    pub(crate) state: Arc<Mutex<State>>,
}

impl InProcessMcp {
    /// Returns the registered MCP tools schema.
    pub fn tools(&self) -> Value {
        serde_json::to_value(self.engine.mcp_list_tools()).unwrap_or_default()
    }

    /// Returns the registered MCP resources schema.
    pub fn resources(&self) -> Value {
        serde_json::to_value(self.engine.mcp_list_resources()).unwrap_or_default()
    }

    /// Reads a resource by URI directly.
    pub fn read_resource(&self, uri: &str) -> Result<Value, String> {
        let contents = self
            .engine
            .mcp_read_resource(uri)
            .map_err(|e| e.to_string())?;
        Ok(json!({
            "contents": [contents],
        }))
    }

    /// Dispatches a JSON-RPC request in-memory, returning a `HandlerResult`.
    pub fn dispatch(&self, request: &Value) -> HandlerResult<Option<String>> {
        self.engine.handle(&request.to_string())
    }

    /// Directly calls a tool by name with the given argument object.
    pub fn call_tool(&self, name: &str, arguments: Value) -> HandlerResult<ToolResult> {
        self.engine.mcp_call_tool_handler(name, arguments)
    }

    /// Returns a shared reference to the underlying documentation and tool engine.
    pub fn engine(&self) -> Arc<DocEngine> {
        self.engine.clone()
    }
}

/// Spawns an in-process MCP server pairing a `Reporter` for the render loop
/// with an `InProcessMcp` handle for client execution.
pub fn in_process(
    slots: Slots,
    store: std::path::PathBuf,
    watching: bool,
    opening: karakuri_environment::Opening,
    slot_policies: karakuri_environment::SlotPolicies,
) -> (Reporter, InProcessMcp) {
    let (tx, rx) = mpsc::sync_channel(QUEUED);
    let (asked, requests) = mpsc::sync_channel(ASKED);
    let (wiring, wires) = mpsc::sync_channel(ASKED);
    let (operating, operations) = mpsc::sync_channel(ASKED);

    let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let dropped = Arc::new(std::sync::atomic::AtomicU64::new(0));

    let state = Arc::new(Mutex::new(State {
        slots,
        store,
        watching,
        opening,
        slot_policies,
        events: rx,
        asked,
        wiring,
        operating,
        recent: Vec::new(),
        dropped: dropped.clone(),
    }));

    let engine = Arc::new(doc_engine_with_state(state.clone()));

    let reporter = Reporter {
        sender: tx,
        requests,
        wires,
        operations,
        dropped,
        shutdown,
        addr: std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
        #[cfg(not(target_arch = "wasm32"))]
        server: None,
    };

    let mcp = InProcessMcp { engine, state };

    (reporter, mcp)
}
