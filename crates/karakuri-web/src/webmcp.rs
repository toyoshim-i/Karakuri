//! WebMCP (Web Model Context Protocol) and in-process browser dispatch bridge.
//! Exposes Karakuri's 14-tool MCP vocabulary directly to modern browser AI contexts
//! (via W3C `document.modelContext` / `navigator.modelContext`) and exposes
//! `window.__karakuri_mcp` for extension and in-browser harness access.

use karakuri_mcp::InProcessMcp;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use winit::event_loop::EventLoopProxy;

/// Yields control to the browser microtask/animation frame queue,
/// allowing the Karakuri render loop to advance without blocking the main thread.
async fn yield_to_browser() {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(win) = web_sys::window() {
            let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
                let _ = resolve.call0(&JsValue::NULL);
            });
            let _ = win.request_animation_frame(cb.unchecked_ref());
        } else {
            let _ = resolve.call0(&JsValue::NULL);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Executes an MCP tool asynchronously against the in-process engine.
pub async fn execute_tool_async(
    mcp: InProcessMcp,
    waker: EventLoopProxy<()>,
    name: String,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut pending = mcp.call_tool(&name, args)?;
    // Wake up the winit event loop immediately to process the queued operation.
    let _ = waker.send_event(());

    let start = web_time::Instant::now();
    let timeout = std::time::Duration::from_secs(10);

    loop {
        match pending.poll_settled()? {
            Some(value) => return Ok(value),
            None => {
                yield_to_browser().await;
            }
        }
        if start.elapsed() > timeout {
            return Err("MCP tool execution timed out waiting for the render loop".into());
        }
    }
}

/// Initializes WebMCP tool registration on the DOM window and document.
pub fn setup_webmcp(mcp: InProcessMcp, waker: EventLoopProxy<()>) -> Result<(), JsValue> {
    let dom_window = web_sys::window().ok_or_else(|| JsValue::from_str("no global window"))?;
    let document = dom_window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;

    // 1. Expose window.__karakuri_mcp for extensions, DevTools console, and internal agent harness
    let karakuri_mcp_obj = js_sys::Object::new();

    let mcp_for_list = mcp.clone();
    let list_tools_closure = Closure::wrap(Box::new(move || -> JsValue {
        let json_str = mcp_for_list.tools().to_string();
        js_sys::JSON::parse(&json_str).unwrap_or(JsValue::NULL)
    }) as Box<dyn FnMut() -> JsValue>);
    js_sys::Reflect::set(
        &karakuri_mcp_obj,
        &"listTools".into(),
        list_tools_closure.as_ref(),
    )?;
    list_tools_closure.forget();

    let mcp_for_call = mcp.clone();
    let waker_for_call = waker.clone();
    let call_tool_closure = Closure::wrap(Box::new(
        move |name: String, args: JsValue| -> js_sys::Promise {
            let mcp = mcp_for_call.clone();
            let waker = waker_for_call.clone();
            let args_json = js_sys::JSON::stringify(&args)
                .ok()
                .and_then(|s| s.as_string())
                .unwrap_or_else(|| "{}".into());
            let args_value: serde_json::Value =
                serde_json::from_str(&args_json).unwrap_or_default();

            wasm_bindgen_futures::future_to_promise(async move {
                match execute_tool_async(mcp, waker, name, args_value).await {
                    Ok(val) => {
                        let json_str = val.to_string();
                        let js_val = js_sys::JSON::parse(&json_str)
                            .unwrap_or_else(|_| JsValue::from_str(&json_str));
                        Ok(js_val)
                    }
                    Err(err) => Err(JsValue::from_str(&err)),
                }
            })
        },
    )
        as Box<dyn FnMut(String, JsValue) -> js_sys::Promise>);
    js_sys::Reflect::set(
        &karakuri_mcp_obj,
        &"callTool".into(),
        call_tool_closure.as_ref(),
    )?;
    call_tool_closure.forget();

    let mcp_for_resources = mcp.clone();
    let list_resources_closure = Closure::wrap(Box::new(move || -> JsValue {
        let json_str = mcp_for_resources.resources().to_string();
        js_sys::JSON::parse(&json_str).unwrap_or(JsValue::NULL)
    }) as Box<dyn FnMut() -> JsValue>);
    js_sys::Reflect::set(
        &karakuri_mcp_obj,
        &"listResources".into(),
        list_resources_closure.as_ref(),
    )?;
    list_resources_closure.forget();

    let mcp_for_read_resource = mcp.clone();
    let read_resource_closure = Closure::wrap(Box::new(move |uri: String| -> JsValue {
        match mcp_for_read_resource.read_resource(&uri) {
            Ok(val) => {
                let json_str = val.to_string();
                js_sys::JSON::parse(&json_str).unwrap_or_else(|_| JsValue::from_str(&json_str))
            }
            Err(err) => JsValue::from_str(&err),
        }
    }) as Box<dyn FnMut(String) -> JsValue>);
    js_sys::Reflect::set(
        &karakuri_mcp_obj,
        &"readResource".into(),
        read_resource_closure.as_ref(),
    )?;
    read_resource_closure.forget();

    js_sys::Reflect::set(&dom_window, &"__karakuri_mcp".into(), &karakuri_mcp_obj)?;
    log::info!("WebMCP: window.__karakuri_mcp registered successfully");

    // 2. Discover standard WebMCP context: document.modelContext or navigator.modelContext
    let model_context_opt = js_sys::Reflect::get(&document, &"modelContext".into())
        .ok()
        .filter(|v| !v.is_undefined() && !v.is_null())
        .or_else(|| {
            let nav = dom_window.navigator();
            js_sys::Reflect::get(&nav, &"modelContext".into())
                .ok()
                .filter(|v| !v.is_undefined() && !v.is_null())
        });

    let Some(model_context) = model_context_opt else {
        log::info!("WebMCP: document.modelContext not present in this browser; accessible via window.__karakuri_mcp");
        return Ok(());
    };

    let register_fn = js_sys::Reflect::get(&model_context, &"registerTool".into())?;
    if !register_fn.is_function() {
        log::warn!("WebMCP: modelContext detected, but registerTool is not a function");
        return Ok(());
    }

    let tools_val = mcp.tools();
    let Some(tool_list) = tools_val.as_array() else {
        return Ok(());
    };

    let mut registered_count = 0;
    for tool_def in tool_list {
        let name_str = tool_def
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let desc_str = tool_def
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let schema_json = tool_def
            .get("inputSchema")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "{}".into());
        let schema_js = js_sys::JSON::parse(&schema_json).unwrap_or(JsValue::NULL);

        let mcp_tool = mcp.clone();
        let waker_tool = waker.clone();
        let name_tool = name_str.clone();

        let execute_fn = Closure::wrap(Box::new(move |args: JsValue| -> js_sys::Promise {
            let mcp = mcp_tool.clone();
            let waker = waker_tool.clone();
            let name = name_tool.clone();
            let args_json = js_sys::JSON::stringify(&args)
                .ok()
                .and_then(|s| s.as_string())
                .unwrap_or_else(|| "{}".into());
            let args_value: serde_json::Value =
                serde_json::from_str(&args_json).unwrap_or_default();

            wasm_bindgen_futures::future_to_promise(async move {
                match execute_tool_async(mcp, waker, name, args_value).await {
                    Ok(val) => {
                        let json_str = val.to_string();
                        let js_val = js_sys::JSON::parse(&json_str)
                            .unwrap_or_else(|_| JsValue::from_str(&json_str));
                        Ok(js_val)
                    }
                    Err(err) => Err(JsValue::from_str(&err)),
                }
            })
        }) as Box<dyn FnMut(JsValue) -> js_sys::Promise>);

        let tool_obj = js_sys::Object::new();
        js_sys::Reflect::set(&tool_obj, &"name".into(), &name_str.clone().into())?;
        js_sys::Reflect::set(&tool_obj, &"description".into(), &desc_str.into())?;
        js_sys::Reflect::set(&tool_obj, &"inputSchema".into(), &schema_js)?;
        js_sys::Reflect::set(&tool_obj, &"execute".into(), execute_fn.as_ref())?;
        execute_fn.forget();

        let register_fn_cast: js_sys::Function = register_fn.clone().unchecked_into();
        if let Err(e) = register_fn_cast.call1(&model_context, &tool_obj) {
            log::warn!("WebMCP: Failed to register tool `{name_str}`: {e:?}");
        } else {
            registered_count += 1;
        }
    }

    log::info!("WebMCP: Successfully registered {registered_count} tools to document.modelContext");
    Ok(())
}
