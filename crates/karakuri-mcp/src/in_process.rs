//! In-process MCP dispatching for WebAssembly, tests, and in-memory agent harnesses.

use serde_json::{json, Value};
use std::sync::{mpsc, Arc, Mutex};

use crate::server::dispatch;
use crate::*;

/// In-process handle to the Karakuri MCP engine.
#[derive(Clone)]
pub struct InProcessMcp {
    pub(crate) state: Arc<Mutex<State>>,
}

impl InProcessMcp {
    /// Returns the registered MCP tools schema.
    pub fn tools(&self) -> Value {
        tools()
    }

    /// Returns the registered MCP resources schema.
    pub fn resources(&self) -> Value {
        resources()
    }

    /// Reads a resource by URI directly.
    pub fn read_resource(&self, uri: &str) -> Result<Value, String> {
        let request = json!({
            "params": {
                "uri": uri,
            }
        });
        read_resource(&request)
    }

    /// Dispatches a JSON-RPC request in-memory, returning a `Pending` result handle.
    pub fn dispatch(&self, request: &Value) -> Result<Pending, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "mcp state mutex is poisoned")?;
        state.drain();
        Ok(dispatch(request, &mut state))
    }

    /// Directly calls a tool by name with the given argument object.
    pub fn call_tool(&self, name: &str, arguments: Value) -> Result<Pending, String> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments,
            }
        });
        self.dispatch(&request)
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

    let reporter = Reporter {
        sender: tx,
        requests,
        wires,
        operations,
        dropped,
        shutdown,
        addr: std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
    };

    let mcp = InProcessMcp { state };

    (reporter, mcp)
}
