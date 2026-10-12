//! WebMCP (Web Model Context Protocol) integration over agent-man webmcp module.

#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use std::time::Duration;

use agent_man::mcp::ToolResult;
#[cfg(target_arch = "wasm32")]
use agent_man::webmcp::{self, WebMcpOptions};
use karakuri_mcp::InProcessMcp;
use wasm_bindgen::prelude::*;
use winit::event_loop::EventLoopProxy;

/// Executes an MCP tool asynchronously against the in-process engine.
#[cfg(target_arch = "wasm32")]
pub async fn execute_tool_async(
    mcp: InProcessMcp,
    waker: EventLoopProxy<()>,
    name: String,
    args: serde_json::Value,
) -> Result<ToolResult, String> {
    let options = WebMcpOptions {
        global_name: None,
        on_call: Some(Rc::new(move || {
            let _ = waker.send_event(());
        })),
        timeout: Duration::from_secs(10),
    };
    webmcp::call_tool(&mcp.engine(), &name, args, &options).await
}

/// Executes an MCP tool asynchronously against the in-process engine (native test fallback).
#[cfg(not(target_arch = "wasm32"))]
pub async fn execute_tool_async(
    mcp: InProcessMcp,
    _waker: EventLoopProxy<()>,
    name: String,
    args: serde_json::Value,
) -> Result<ToolResult, String> {
    match mcp.call_tool(&name, args) {
        agent_man::mcp::HandlerResult::Ready(res) => Ok(res),
        agent_man::mcp::HandlerResult::Pending(p) => Ok(p.settled()),
    }
}

/// Initializes WebMCP tool registration on the DOM window and document.
#[cfg(target_arch = "wasm32")]
pub fn setup_webmcp(mcp: InProcessMcp, waker: EventLoopProxy<()>) -> Result<(), JsValue> {
    let options = WebMcpOptions {
        global_name: Some("__karakuri_mcp".to_string()),
        on_call: Some(Rc::new(move || {
            let _ = waker.send_event(());
        })),
        timeout: Duration::from_secs(10),
    };
    let engine = Rc::new((*mcp.engine()).clone());
    webmcp::install(engine, options)?;
    Ok(())
}

/// Initializes WebMCP tool registration on native (no-op fallback).
#[cfg(not(target_arch = "wasm32"))]
pub fn setup_webmcp(_mcp: InProcessMcp, _waker: EventLoopProxy<()>) -> Result<(), JsValue> {
    Ok(())
}
