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

    let tools_array_opt = if let Some(arr) = mcp_tools.as_array() {
        Some(arr)
    } else {
        mcp_tools.get("tools").and_then(|t| t.as_array())
    };

    if let Some(tools_array) = tools_array_opt {
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

/// Extracts tool calls from unstructured or markdown text when models emit
/// raw JSON blocks instead of native OpenAI `tool_calls` deltas.
/// Returns the extracted tool calls and the remaining cleaned text with tool call blocks removed.
pub fn extract_text_tool_calls_and_clean(
    text: &str,
    valid_tool_names: &[String],
) -> (Vec<(String, String)>, Option<String>) {
    let mut results = Vec::new();
    let mut removal_ranges = Vec::new();

    // 1. Look for ```json ... ``` or ``` ... ``` blocks
    let mut search_idx = 0;
    while let Some(start_tick) = text[search_idx..].find("```") {
        let abs_start = search_idx + start_tick;
        let inner_start = abs_start + 3;
        let content_start = if let Some(newline_pos) = text[inner_start..].find('\n') {
            inner_start + newline_pos + 1
        } else {
            inner_start
        };

        if let Some(end_tick) = text[content_start..].find("```") {
            let block = text[content_start..content_start + end_tick].trim();
            if let Some(parsed) = try_parse_candidate_tool_call(block, valid_tool_names) {
                results.extend(parsed);
                let abs_end = content_start + end_tick + 3;
                removal_ranges.push((abs_start, abs_end));
            }
            search_idx = content_start + end_tick + 3;
        } else {
            break;
        }
    }

    // 2. Look for <tool_call> ... </tool_call> tags
    search_idx = 0;
    while let Some(start_tag) = text[search_idx..].find("<tool_call>") {
        let abs_start = search_idx + start_tag;
        let content_start = abs_start + 11;
        if let Some(end_tag) = text[content_start..].find("</tool_call>") {
            let block = text[content_start..content_start + end_tag].trim();
            if let Some(parsed) = try_parse_candidate_tool_call(block, valid_tool_names) {
                results.extend(parsed);
                let abs_end = content_start + end_tag + 12;
                removal_ranges.push((abs_start, abs_end));
            }
            search_idx = content_start + end_tag + 12;
        } else {
            break;
        }
    }

    // 3. If no fenced blocks yielded valid calls, try parsing the entire trimmed text or raw {...} substring
    if results.is_empty() {
        if let Some(parsed) = try_parse_candidate_tool_call(text.trim(), valid_tool_names) {
            results.extend(parsed);
            removal_ranges.push((0, text.len()));
        } else if let (Some(first_brace), Some(last_brace)) = (text.find('{'), text.rfind('}')) {
            if first_brace < last_brace {
                let sub = &text[first_brace..=last_brace];
                if let Some(parsed) = try_parse_candidate_tool_call(sub, valid_tool_names) {
                    results.extend(parsed);
                    removal_ranges.push((first_brace, last_brace + 1));
                }
            }
        }
    }

    if results.is_empty() {
        let clean = text.trim();
        let ret = if clean.is_empty() {
            None
        } else {
            Some(clean.to_string())
        };
        return (results, ret);
    }

    // Remove the ranges from text
    let mut cleaned = String::with_capacity(text.len());
    let mut last_idx = 0;
    for (start, end) in removal_ranges {
        if start >= last_idx {
            cleaned.push_str(&text[last_idx..start]);
            last_idx = end;
        }
    }
    if last_idx < text.len() {
        cleaned.push_str(&text[last_idx..]);
    }

    let trimmed = cleaned.trim();
    let ret = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    };

    (results, ret)
}

/// Extracts tool calls from unstructured or markdown text when models emit
/// raw JSON blocks instead of native OpenAI `tool_calls` deltas.
pub fn extract_text_tool_calls(text: &str, valid_tool_names: &[String]) -> Vec<(String, String)> {
    extract_text_tool_calls_and_clean(text, valid_tool_names).0
}

fn try_parse_candidate_tool_call(
    json_str: &str,
    valid_tool_names: &[String],
) -> Option<Vec<(String, String)>> {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return None;
    };

    let mut found = Vec::new();

    if let Some(arr) = val.as_array() {
        for item in arr {
            if let Some((name, args)) = parse_single_tool_call_value(item, valid_tool_names) {
                found.push((name, args));
            }
        }
    } else if let Some((name, args)) = parse_single_tool_call_value(&val, valid_tool_names) {
        found.push((name, args));
    }

    if found.is_empty() {
        None
    } else {
        Some(found)
    }
}

fn parse_single_tool_call_value(
    val: &serde_json::Value,
    valid_tool_names: &[String],
) -> Option<(String, String)> {
    let obj = val.as_object()?;

    let name_opt = obj
        .get("name")
        .or_else(|| obj.get("tool"))
        .or_else(|| obj.get("action"))
        .or_else(|| obj.get("function"))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                Some(s.to_string())
            } else if let Some(fn_obj) = v.as_object() {
                fn_obj
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        });

    let name = name_opt?;
    if !valid_tool_names.iter().any(|v| v == &name) {
        return None;
    }

    let args_val = if let Some(args) = obj
        .get("arguments")
        .or_else(|| obj.get("parameters"))
        .or_else(|| obj.get("args"))
        .or_else(|| obj.get("input"))
    {
        if let Some(s) = args.as_str() {
            serde_json::from_str(s).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            args.clone()
        }
    } else if name == "operate" && obj.contains_key("operation") {
        val.clone()
    } else {
        let mut clean = obj.clone();
        clean.remove("name");
        clean.remove("tool");
        clean.remove("action");
        clean.remove("function");
        serde_json::Value::Object(clean)
    };

    Some((name, args_val.to_string()))
}

/// Extracts the underlying payload text from an MCP JSON-RPC response.
pub fn extract_mcp_result_text(result_val: &serde_json::Value) -> String {
    // 1. If wrapped in JSON-RPC: result.content
    if let Some(res) = result_val.get("result") {
        if let Some(content_arr) = res.get("content").and_then(|c| c.as_array()) {
            let mut texts = Vec::new();
            for item in content_arr {
                if let Some(t) = item.get("text").and_then(|s| s.as_str()) {
                    texts.push(t.to_string());
                }
            }
            if !texts.is_empty() {
                return texts.join("\n");
            }
        }
        return res.to_string();
    }
    // 2. If it is already a content array:
    if let Some(content_arr) = result_val.get("content").and_then(|c| c.as_array()) {
        let mut texts = Vec::new();
        for item in content_arr {
            if let Some(t) = item.get("text").and_then(|s| s.as_str()) {
                texts.push(t.to_string());
            }
        }
        if !texts.is_empty() {
            return texts.join("\n");
        }
    }
    result_val.to_string()
}

/// Prepares messages for sending to the LLM. Messages remain strictly immutable
/// to ensure prefix caching (prompt caching / KV cache) hits across multi-turn interactions.
pub fn prepare_messages(messages: &[serde_json::Value]) -> Vec<serde_json::Value> {
    messages.to_vec()
}

/// Executes a chat completion query against the configured LLM endpoint, streaming tokens
/// and dispatching any emitted `tool_calls` in-process. Automatically falls back to chat-only
/// mode if the model does not support tools schema.
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
    let mut enable_tools = true;

    while round < MAX_ROUNDS {
        round += 1;

        let outgoing_messages = prepare_messages(messages);

        let mut request_body = json!({
            "model": config.model,
            "messages": outgoing_messages,
            "stream": true,
        });

        if enable_tools {
            if let Some(obj) = request_body.as_object_mut() {
                obj.insert("tools".to_string(), tools_schema.clone());
            }
        }

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

            let lower_body = body.to_lowercase();
            // If the failure was due to tools/function calling or schema, retry in chat-only mode
            // without modifying message history prefixes.
            if enable_tools
                && (status == 400
                    || lower_body.contains("tool")
                    || lower_body.contains("function")
                    || lower_body.contains("support")
                    || lower_body.contains("schema"))
            {
                enable_tools = false;
                on_output(
                    "\r\n\x1b[36m[Notice: Model does not support function calling tools; falling back to direct chat mode]\x1b[0m\r\n",
                );
                continue;
            }

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

        // Resolve tool calls: either from native streaming delta `tool_calls`,
        // or extracted from markdown/raw JSON blocks in `assistant_content` (for models that emit JSON in text).
        let mut resolved_tool_calls: Vec<(String, String, String)> =
            tool_calls_map.into_values().collect();

        let mut cleaned_assistant_text: Option<String> = if assistant_content.trim().is_empty() {
            None
        } else {
            Some(assistant_content.trim().to_string())
        };

        if resolved_tool_calls.is_empty() && enable_tools {
            let valid_names: Vec<String> = mcp
                .tools()
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| {
                            t.get("name")
                                .and_then(|n| n.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect()
                })
                .unwrap_or_default();

            let (fallback_calls, clean_text) =
                extract_text_tool_calls_and_clean(&assistant_content, &valid_names);
            cleaned_assistant_text = clean_text;
            for (idx, (fn_name, args_str)) in fallback_calls.into_iter().enumerate() {
                resolved_tool_calls.push((format!("call_{idx}"), fn_name, args_str));
            }
        }

        // If tools were called, execute them and append to conversation
        if !resolved_tool_calls.is_empty() {
            let mut tool_calls_json = Vec::new();
            for (call_id, fn_name, args_str) in &resolved_tool_calls {
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
                "content": match cleaned_assistant_text {
                    Some(text) => json!(text),
                    None => serde_json::Value::Null,
                },
                "tool_calls": tool_calls_json,
            }));

            for (call_id, fn_name, args_str) in resolved_tool_calls {
                let id = if call_id.is_empty() {
                    "call_1".to_string()
                } else {
                    call_id
                };
                on_output(&format!("\r\n\x1b[36m⚡ Executing {fn_name}...\x1b[0m\r\n"));

                let args_val: serde_json::Value =
                    serde_json::from_str(&args_str).unwrap_or_else(|_| json!({}));

                match execute_tool_async(mcp.clone(), waker.clone(), fn_name.clone(), args_val)
                    .await
                {
                    Ok(result) => {
                        let payload_text = extract_mcp_result_text(&result);
                        #[cfg(target_arch = "wasm32")]
                        {
                            web_sys::console::log_2(
                                &wasm_bindgen::JsValue::from_str(&format!(
                                    "[Karakuri Tool: {fn_name}]"
                                )),
                                &wasm_bindgen::JsValue::from_str(&payload_text),
                            );
                        }
                        log::info!("[Karakuri Tool: {fn_name}] => {payload_text}");

                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": id,
                            "name": fn_name,
                            "content": payload_text,
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
