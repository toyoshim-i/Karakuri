//! Asynchronous port probing and model discovery on localhost.

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredModel {
    pub server_name: String,
    pub base_url: String,
    pub model_id: String,
}

#[derive(Debug, Clone)]
struct TargetServer {
    name: &'static str,
    base_url: &'static str,
    probe_path: &'static str,
    is_ollama_tags: bool,
}

const SERVERS: &[TargetServer] = &[
    TargetServer {
        name: "Ollama",
        base_url: "http://localhost:11434",
        probe_path: "/api/tags",
        is_ollama_tags: true,
    },
    TargetServer {
        name: "Ollama",
        base_url: "http://127.0.0.1:11434",
        probe_path: "/api/tags",
        is_ollama_tags: true,
    },
    TargetServer {
        name: "LM Studio",
        base_url: "http://localhost:1234",
        probe_path: "/v1/models",
        is_ollama_tags: false,
    },
    TargetServer {
        name: "LM Studio",
        base_url: "http://127.0.0.1:1234",
        probe_path: "/v1/models",
        is_ollama_tags: false,
    },
    TargetServer {
        name: "vLLM / llama.cpp",
        base_url: "http://localhost:8000",
        probe_path: "/v1/models",
        is_ollama_tags: false,
    },
    TargetServer {
        name: "LocalAI / llama.cpp",
        base_url: "http://localhost:8080",
        probe_path: "/v1/models",
        is_ollama_tags: false,
    },
];

/// Probes a single target server endpoint and extracts available model IDs.
async fn probe_server(target: &TargetServer) -> Result<Vec<DiscoveredModel>, String> {
    let window = web_sys::window().ok_or_else(|| "no global window".to_string())?;
    let url = format!("{}{}", target.base_url, target.probe_path);

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("request creation error: {e:?}"))?;

    let resp_val = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("fetch error: {e:?}"))?;

    let resp: Response = resp_val
        .dyn_into()
        .map_err(|_| "response cast error".to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP status {}", resp.status()));
    }

    let json_val = JsFuture::from(resp.json().map_err(|e| format!("json error: {e:?}"))?)
        .await
        .map_err(|e| format!("json parse error: {e:?}"))?;

    let json_str = js_sys::JSON::stringify(&json_val)
        .map_err(|e| format!("stringify error: {e:?}"))?
        .as_string()
        .unwrap_or_default();

    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("serde parse error: {e}"))?;

    let mut models = Vec::new();

    if target.is_ollama_tags {
        // Ollama format: { "models": [ { "name": "llama3.2:latest" } ] }
        if let Some(list) = parsed.get("models").and_then(|m| m.as_array()) {
            for item in list {
                if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                    models.push(DiscoveredModel {
                        server_name: target.name.to_string(),
                        base_url: target.base_url.to_string(),
                        model_id: name.to_string(),
                    });
                }
            }
        }
    }

    // Standard OpenAI format fallback: { "data": [ { "id": "model_id" } ] }
    if models.is_empty() {
        if let Some(list) = parsed.get("data").and_then(|d| d.as_array()) {
            for item in list {
                if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                    models.push(DiscoveredModel {
                        server_name: target.name.to_string(),
                        base_url: target.base_url.to_string(),
                        model_id: id.to_string(),
                    });
                }
            }
        }
    }

    if models.is_empty() {
        Err("no models found in response".to_string())
    } else {
        Ok(models)
    }
}

/// Discovers available local LLMs by scanning well-known localhost ports.
/// Calls `on_progress` with progress log lines.
pub async fn discover_local_models<F>(mut on_progress: F) -> Vec<DiscoveredModel>
where
    F: FnMut(&str),
{
    let mut discovered = Vec::new();
    let mut visited_urls = std::collections::HashSet::new();

    for target in SERVERS {
        // Skip duplicate ports between localhost and 127.0.0.1 if base port succeeded
        let port_key = target
            .base_url
            .split(':')
            .nth(2)
            .unwrap_or_default()
            .to_string();
        if visited_urls.contains(&port_key) {
            continue;
        }

        match probe_server(target).await {
            Ok(models) => {
                on_progress(&format!(
                    "  \x1b[32m✔\x1b[0m {} ({}): {} models found\r\n",
                    target.name,
                    target.base_url,
                    models.len()
                ));
                visited_urls.insert(port_key);
                discovered.extend(models);
            }
            Err(_) => {
                // Not running or CORS not configured, silently skip
            }
        }
    }

    discovered
}
