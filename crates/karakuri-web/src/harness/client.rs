//! OpenAI-compatible streaming client with In-Process MCP tool execution.

use karakuri_mcp::InProcessMcp;
use serde_json::json;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};
use winit::event_loop::EventLoopProxy;

use super::config::HarnessConfig;
use crate::webmcp::execute_tool_async;

/// Converts Karakuri MCP tool definitions to OpenAI-compatible function calling schemas.
pub fn mcp_tools_to_openai(mcp_tools: &serde_json::Value) -> serde_json::Value {
    let mut openai_tools = Vec::new();

    if let Some(tools_array) = mcp_tools.get("tools").and_then(|t| t.as_array()) {
        for t in tools_array {
            let name = t.get("name").and_then(|n| n.as_str()).unwrap_or_default();
            let desc = t
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or_default();
            let params = t.get("inputSchema").cloned().unwrap_or_else(|| {
                json!({
                    "type": "object",
                    "properties": {}
                })
            });

            openai_tools.push(json!({
                "type": "function",
                "function": {
                    "name": name,
                    "description": desc,
                    "parameters": params,
                }
            }));
        }
    }

    json!(openai_tools)
}

/// Executes a chat completion query against the configured LLM endpoint, streaming tokens
/// and dispatching any emitted `tool_calls` in-process.
pub async fn run_agent_loop<F>(
    config: HarnessConfig,
    mcp: InProcessMcp,
    waker: EventLoopProxy<()>,
    messages: &mut Vec<serde_json::Value>,
    mut on_output: F,
) -> Result<(), String>
where
    F: FnMut(&str) + Clone + 'static,
{
    let tools_schema = mcp_tools_to_openai(&mcp.tools());
    let mut round = 0;
    const MAX_ROUNDS: usize = 5;

    while round < MAX_ROUNDS {
        round += 1;

        let request_body = json!({
            "model": config.model,
            "messages": messages,
            "tools": tools_schema,
            "stream": true,
        });

        let endpoint_url = format!(
            "{}/v1/chat/completions",
            config.endpoint.trim_end_matches('/')
        );

        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_mode(RequestMode::Cors);
        opts.set_body(&wasm_bindgen::JsValue::from_str(&request_body.to_string()));

        let headers = Headers::new().map_err(|e| format!("headers error: {e:?}"))?;
        headers
            .set("Content-Type", "application/json")
            .map_err(|e| format!("header set error: {e:?}"))?;
        if let Some(ref key) = config.api_key {
            if !key.trim().is_empty() {
                headers
                    .set("Authorization", &format!("Bearer {}", key.trim()))
                    .map_err(|e| format!("header set error: {e:?}"))?;
            }
        }
        opts.set_headers(&headers);

        let window = web_sys::window().ok_or_else(|| "no global window".to_string())?;
        let request = Request::new_with_str_and_init(&endpoint_url, &opts)
            .map_err(|e| format!("request error: {e:?}"))?;

        let resp_val = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| format!("fetch error: {e:?} (check server connection and CORS)"))?;

        let resp: Response = resp_val
            .dyn_into()
            .map_err(|_| "response cast error".to_string())?;

        if !resp.ok() {
            let status = resp.status();
            let body = JsFuture::from(resp.text().map_err(|e| format!("text error: {e:?}"))?)
                .await
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            return Err(format!("LLM HTTP error {status}: {body}"));
        }

        let body = resp
            .body()
            .ok_or_else(|| "response has no body".to_string())?;
        let reader: web_sys::ReadableStreamDefaultReader = body
            .get_reader()
            .dyn_into()
            .map_err(|_| "get_reader error".to_string())?;

        let mut assistant_content = String::new();
        let mut tool_calls_map: std::collections::BTreeMap<usize, (String, String, String)> =
            std::collections::BTreeMap::new(); // index -> (call_id, function_name, arguments_accum)

        let mut line_buffer = String::new();

        loop {
            let chunk_val = JsFuture::from(reader.read())
                .await
                .map_err(|e| format!("stream read error: {e:?}"))?;

            let done = js_sys::Reflect::get(&chunk_val, &"done".into())
                .ok()
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            if done {
                break;
            }

            let value = js_sys::Reflect::get(&chunk_val, &"value".into())
                .map_err(|e| format!("chunk value error: {e:?}"))?;
            let uint8_array = js_sys::Uint8Array::new(&value);
            let mut bytes = vec![0u8; uint8_array.length() as usize];
            uint8_array.copy_to(&mut bytes);

            if let Ok(text) = String::from_utf8(bytes) {
                line_buffer.push_str(&text);

                while let Some(pos) = line_buffer.find('\n') {
                    let line = line_buffer[..pos].trim().to_string();
                    line_buffer = line_buffer[pos + 1..].to_string();

                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            break;
                        }

                        if let Ok(chunk_json) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(choices) =
                                chunk_json.get("choices").and_then(|c| c.as_array())
                            {
                                if let Some(choice) = choices.first() {
                                    if let Some(delta) = choice.get("delta") {
                                        // 1. Text token
                                        if let Some(token) =
                                            delta.get("content").and_then(|c| c.as_str())
                                        {
                                            assistant_content.push_str(token);
                                            on_output(token);
                                        }

                                        // 2. Tool calls delta
                                        if let Some(tools) =
                                            delta.get("tool_calls").and_then(|t| t.as_array())
                                        {
                                            for tc in tools {
                                                let index = tc
                                                    .get("index")
                                                    .and_then(|i| i.as_u64())
                                                    .unwrap_or(0)
                                                    as usize;
                                                let entry = tool_calls_map
                                                    .entry(index)
                                                    .or_insert_with(|| {
                                                        (
                                                            String::new(),
                                                            String::new(),
                                                            String::new(),
                                                        )
                                                    });

                                                if let Some(id) =
                                                    tc.get("id").and_then(|i| i.as_str())
                                                {
                                                    entry.0.push_str(id);
                                                }
                                                if let Some(func) = tc.get("function") {
                                                    if let Some(fn_name) =
                                                        func.get("name").and_then(|n| n.as_str())
                                                    {
                                                        entry.1.push_str(fn_name);
                                                    }
                                                    if let Some(args_chunk) = func
                                                        .get("arguments")
                                                        .and_then(|a| a.as_str())
                                                    {
                                                        entry.2.push_str(args_chunk);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // If tools were called, execute them and append to conversation
        if !tool_calls_map.is_empty() {
            let mut tool_calls_json = Vec::new();
            for (call_id, fn_name, args_str) in tool_calls_map.values() {
                let id = if call_id.is_empty() {
                    "call_1".to_string()
                } else {
                    call_id.clone()
                };
                tool_calls_json.push(json!({
                    "id": id,
                    "type": "function",
                    "function": {
                        "name": fn_name,
                        "arguments": args_str,
                    }
                }));
            }

            messages.push(json!({
                "role": "assistant",
                "content": if assistant_content.is_empty() { serde_json::Value::Null } else { json!(assistant_content) },
                "tool_calls": tool_calls_json,
            }));

            for (call_id, fn_name, args_str) in tool_calls_map.into_values() {
                let id = if call_id.is_empty() {
                    "call_1".to_string()
                } else {
                    call_id.clone()
                };
                on_output(&format!(
                    "\r\n\x1b[33m⚡ Executing tool:\x1b[0m \x1b[1;37m{fn_name}\x1b[0m({args_str})...\r\n"
                ));

                let args_val: serde_json::Value =
                    serde_json::from_str(&args_str).unwrap_or_else(|_| json!({}));

                match execute_tool_async(mcp.clone(), waker.clone(), fn_name.clone(), args_val)
                    .await
                {
                    Ok(result) => {
                        let res_str = result.to_string();
                        on_output(&format!("  \x1b[32m✔ Result:\x1b[0m {res_str}\r\n"));
                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": id,
                            "name": fn_name,
                            "content": res_str,
                        }));
                    }
                    Err(e) => {
                        on_output(&format!("  \x1b[31m✖ Tool error:\x1b[0m {e}\r\n"));
                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": id,
                            "name": fn_name,
                            "content": json!({ "error": e }).to_string(),
                        }));
                    }
                }
            }

            // Loop continues to let the LLM generate its final response with the tool results
            on_output("\r\n");
            continue;
        }

        // No tool calls; assistant response is complete
        if !assistant_content.is_empty() {
            messages.push(json!({
                "role": "assistant",
                "content": assistant_content,
            }));
        }
        break;
    }

    Ok(())
}
