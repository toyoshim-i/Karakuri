//! MCP server initialization and lifecycle management over HTTP.

use std::sync::mpsc;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use crate::resources::doc_engine_with_state;
use crate::*;

/// Serves MCP at `addr` (e.g. "127.0.0.1:4040" or "4040") until shutdown or dropped.
///
/// Returns the [`Reporter`] handle for the render loop to send notifications and events.
/// The server runs on dedicated threads decoupled from frame rendering.
pub fn serve_at(
    addr: &str,
    slots: Slots,
    store: std::path::PathBuf,
    watching: bool,
    opening: karakuri_environment::Opening,
    slot_policies: karakuri_environment::SlotPolicies,
) -> Result<Reporter, String> {
    if slots.count() == 0 {
        return Err("this run has no procedure files to serve — see `--load-set`".into());
    }
    let target = if addr.contains(':') {
        addr.to_string()
    } else {
        format!("127.0.0.1:{addr}")
    };

    let (tx, rx) = mpsc::sync_channel(QUEUED);
    let (asked, requests) = mpsc::sync_channel(ASKED);
    let (wiring, wires) = mpsc::sync_channel(ASKED);
    let (operating, operations) = mpsc::sync_channel(ASKED);
    let (capturing, captures) = mpsc::sync_channel(ASKED);

    let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let dropped = Arc::new(std::sync::atomic::AtomicU64::new(0));

    let state = Arc::new(std::sync::Mutex::new(State {
        slots,
        store,
        watching,
        opening,
        slot_policies,
        events: rx,
        asked,
        wiring,
        operating,
        capturing,
        recent: Vec::new(),
        dropped: dropped.clone(),
    }));

    #[cfg(not(target_arch = "wasm32"))]
    {
        let engine = doc_engine_with_state(state);
        let server = engine
            .serve_http(&target)
            .map_err(|e| format!("addr {target}: {e}"))?;
        let bound = server.local_addr();

        Ok(Reporter {
            sender: tx,
            requests,
            wires,
            operations,
            captures,
            dropped,
            shutdown,
            addr: bound,
            server: Some(server),
        })
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (
            state, target, tx, requests, wires, operations, captures, shutdown, dropped,
        );
        Err("HTTP MCP server is not supported on this target".into())
    }
}

/// Serves MCP on `port` over loopback until the process ends.
///
/// Returns the [`Reporter`] handle for the render loop to send notifications and events.
/// The server runs on dedicated threads decoupled from frame rendering.
pub fn serve(
    port: u16,
    slots: Slots,
    store: std::path::PathBuf,
    watching: bool,
    opening: karakuri_environment::Opening,
    slot_policies: karakuri_environment::SlotPolicies,
) -> Result<Reporter, String> {
    serve_at(
        &format!("127.0.0.1:{port}"),
        slots,
        store,
        watching,
        opening,
        slot_policies,
    )
}
