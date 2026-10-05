//! In-Process Agent Harness orchestrator for WebAssembly.
//! Provides terminal prompt interaction, localhost model detection, and streaming MCP tool calling.

pub mod client;
pub mod config;
pub mod menu;
pub mod probe;

#[cfg(test)]
mod tests;

use std::io::Write;
use std::sync::{Arc, Mutex};

use karakuri_console::view::prompt::ansi::Scrollback;
use karakuri_console::view::prompt::TerminalSession;
use karakuri_mcp::InProcessMcp;
use serde_json::json;
use winit::event_loop::EventLoopProxy;

use self::config::HarnessConfig;
use self::menu::{MenuAction, ModelMenu};

const PROMPT_LABEL: &str = "\x1b[1;35mkarakuri\x1b[0m> ";

const SYSTEM_PROMPT: &str = r#"You are the Karakuri AI VJ Assistant running directly inside the browser.
You control a real-time procedural WebGPU visual performance engine with 14 in-process MCP tools.
You can read slots, wire parameters, transition decks, inspect performance costs, and execute operations.
When the user asks you to operate the mixer, change BPM, swap procedures, or alter visuals, call the appropriate Karakuri tool.
Respond concisely and helpfully in the user's language."#;

pub struct Harness {
    config: Arc<Mutex<HarnessConfig>>,
    mcp: InProcessMcp,
    waker: EventLoopProxy<()>,
    scrollback: Arc<Mutex<Scrollback>>,
    messages: Arc<Mutex<Vec<serde_json::Value>>>,
    input_buffer: Arc<Mutex<String>>,
    active_menu: Arc<Mutex<Option<ModelMenu>>>,
    is_busy: Arc<Mutex<bool>>,
}

impl Harness {
    pub fn new(
        mcp: InProcessMcp,
        waker: EventLoopProxy<()>,
        scrollback: Arc<Mutex<Scrollback>>,
    ) -> Self {
        let config = HarnessConfig::load();
        let initial_messages = vec![json!({
            "role": "system",
            "content": SYSTEM_PROMPT,
        })];

        Self {
            config: Arc::new(Mutex::new(config)),
            mcp,
            waker,
            scrollback,
            messages: Arc::new(Mutex::new(initial_messages)),
            input_buffer: Arc::new(Mutex::new(String::new())),
            active_menu: Arc::new(Mutex::new(None)),
            is_busy: Arc::new(Mutex::new(false)),
        }
    }

    /// Prints the initial welcome banner and prompt to the terminal.
    pub fn init_banner(&self) {
        let cfg = self.config.lock().unwrap().clone();
        let mut out = String::new();
        out.push_str(
            "\r\n\x1b[1;36m┌────────────────────────────────────────────────────────┐\x1b[0m\r\n",
        );
        out.push_str(
            "\x1b[1;36m│   Karakuri Web Agent Harness (In-Process Client)       │\x1b[0m\r\n",
        );
        out.push_str(
            "\x1b[1;36m└────────────────────────────────────────────────────────┘\x1b[0m\r\n",
        );
        out.push_str(&format!(
            "  Endpoint: \x1b[33m{}\x1b[0m | Model: \x1b[1;32m{}\x1b[0m\r\n",
            cfg.endpoint, cfg.model
        ));
        out.push_str("  Commands: \x1b[1;32m/model\x1b[0m (auto-detect local LLMs) · \x1b[1;32m/help\x1b[0m · \x1b[1;32m/clear\x1b[0m\r\n\r\n");
        out.push_str(PROMPT_LABEL);

        if let Ok(mut sb) = self.scrollback.lock() {
            sb.push_str(&out);
        }
    }

    /// Handles incoming raw keyboard bytes from egui.
    pub fn handle_bytes(&self, bytes: &[u8]) {
        // 1. If busy with LLM request, only Ctrl+C is accepted
        if *self.is_busy.lock().unwrap() {
            if bytes == b"\x03" {
                if let Ok(mut sb) = self.scrollback.lock() {
                    sb.push_str("\r\n\x1b[31m[Cancelled]\x1b[0m\r\n");
                    sb.push_str(PROMPT_LABEL);
                }
                *self.is_busy.lock().unwrap() = false;
            }
            return;
        }

        // 2. If menu is active, delegate to ModelMenu
        let mut menu_opt = self.active_menu.lock().unwrap();
        if let Some(ref mut menu) = *menu_opt {
            let action = menu.handle_key(bytes);
            let current_cfg = self.config.lock().unwrap().clone();

            match action {
                MenuAction::Redraw => {
                    let update = menu.render_update(&current_cfg);
                    if let Ok(mut sb) = self.scrollback.lock() {
                        sb.push_str(&update);
                    }
                }
                MenuAction::Selected(model) => {
                    {
                        let mut cfg = self.config.lock().unwrap();
                        cfg.endpoint = model.base_url.clone();
                        cfg.model = model.model_id.clone();
                        cfg.save();
                    }
                    *menu_opt = None;

                    if let Ok(mut sb) = self.scrollback.lock() {
                        sb.push_str(&format!(
                            "\r\n\x1b[1;32m✔ Selected model:\x1b[0m \x1b[1;37m{}\x1b[0m ({})\r\n\r\n{}",
                            model.model_id, model.base_url, PROMPT_LABEL
                        ));
                    }
                }
                MenuAction::Cancelled => {
                    *menu_opt = None;
                    if let Ok(mut sb) = self.scrollback.lock() {
                        sb.push_str(&format!(
                            "\r\n\x1b[2m[Model selection cancelled]\x1b[0m\r\n\r\n{}",
                            PROMPT_LABEL
                        ));
                    }
                }
                MenuAction::None => {}
            }
            return;
        }
        drop(menu_opt);

        // 3. Normal line editing
        if bytes == b"\r" || bytes == b"\n" {
            let line = {
                let mut buf = self.input_buffer.lock().unwrap();
                let taken = buf.clone();
                buf.clear();
                taken
            };
            if let Ok(mut sb) = self.scrollback.lock() {
                sb.push_str("\r\n");
            }
            self.execute_line(&line);
        } else if bytes == b"\x7f" || bytes == b"\x08" {
            // Backspace
            let mut buf = self.input_buffer.lock().unwrap();
            if !buf.is_empty() {
                buf.pop();
                if let Ok(mut sb) = self.scrollback.lock() {
                    sb.push_str("\x08 \x08");
                }
            }
        } else if bytes == b"\x03" {
            // Ctrl+C
            self.input_buffer.lock().unwrap().clear();
            if let Ok(mut sb) = self.scrollback.lock() {
                sb.push_str("^C\r\n");
                sb.push_str(PROMPT_LABEL);
            }
        } else if bytes == b"\x0c" {
            // Ctrl+L (Clear screen)
            if let Ok(mut sb) = self.scrollback.lock() {
                sb.push_str("\x1b[2J\x1b[H");
                sb.push_str(PROMPT_LABEL);
                let buf = self.input_buffer.lock().unwrap();
                sb.push_str(&buf);
            }
        } else if let Ok(s) = std::str::from_utf8(bytes) {
            // Printable text
            if !s.chars().any(|c| c.is_control()) {
                self.input_buffer.lock().unwrap().push_str(s);
                if let Ok(mut sb) = self.scrollback.lock() {
                    sb.push_str(s);
                }
            }
        }
    }

    fn execute_line(&self, raw_line: &str) {
        let line = raw_line.trim();
        if line.is_empty() {
            if let Ok(mut sb) = self.scrollback.lock() {
                sb.push_str(PROMPT_LABEL);
            }
            return;
        }

        if line == "/model" {
            self.run_model_detection();
        } else if line == "/help" {
            if let Ok(mut sb) = self.scrollback.lock() {
                sb.push_str("\x1b[1;36mAvailable Commands:\x1b[0m\r\n");
                sb.push_str("  \x1b[1;32m/model\x1b[0m                    Probe localhost ports and select a local LLM\r\n");
                sb.push_str("  \x1b[1;32m/config endpoint <url>\x1b[0m    Set custom LLM API endpoint URL\r\n");
                sb.push_str(
                    "  \x1b[1;32m/config model <name>\x1b[0m      Set custom model name\r\n",
                );
                sb.push_str("  \x1b[1;32m/config key <api-key>\x1b[0m     Set API key for cloud providers\r\n");
                sb.push_str("  \x1b[1;32m/clear\x1b[0m                    Clear the terminal scrollback\r\n");
                sb.push_str("  \x1b[1;32m/reset\x1b[0m                    Reset conversation message history\r\n\r\n");
                sb.push_str(PROMPT_LABEL);
            }
        } else if line.starts_with("/config") {
            self.handle_config_command(line);
        } else if line == "/clear" {
            if let Ok(mut sb) = self.scrollback.lock() {
                sb.push_str("\x1b[2J\x1b[H");
                sb.push_str(PROMPT_LABEL);
            }
        } else if line == "/reset" {
            let mut msgs = self.messages.lock().unwrap();
            msgs.clear();
            msgs.push(json!({
                "role": "system",
                "content": SYSTEM_PROMPT,
            }));
            if let Ok(mut sb) = self.scrollback.lock() {
                sb.push_str("\x1b[2m[Conversation history reset]\x1b[0m\r\n\r\n");
                sb.push_str(PROMPT_LABEL);
            }
        } else {
            // Normal prompt execution
            self.run_query(line.to_string());
        }
    }

    fn handle_config_command(&self, line: &str) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut cfg = self.config.lock().unwrap();

        if parts.len() == 1 {
            if let Ok(mut sb) = self.scrollback.lock() {
                sb.push_str(&format!(
                    "Current Configuration:\r\n  Endpoint: \x1b[33m{}\x1b[0m\r\n  Model:    \x1b[1;32m{}\x1b[0m\r\n  Key:      \x1b[2m{}\x1b[0m\r\n\r\n{}",
                    cfg.endpoint,
                    cfg.model,
                    if cfg.api_key.is_some() { "[set]" } else { "[none]" },
                    PROMPT_LABEL
                ));
            }
            return;
        }

        match parts.get(1).copied() {
            Some("endpoint") => {
                if let Some(val) = parts.get(2) {
                    cfg.endpoint = val.to_string();
                    cfg.save();
                    if let Ok(mut sb) = self.scrollback.lock() {
                        sb.push_str(&format!(
                            "✔ Endpoint set to: \x1b[33m{}\x1b[0m\r\n\r\n{}",
                            cfg.endpoint, PROMPT_LABEL
                        ));
                    }
                }
            }
            Some("model") => {
                if let Some(val) = parts.get(2) {
                    cfg.model = val.to_string();
                    cfg.save();
                    if let Ok(mut sb) = self.scrollback.lock() {
                        sb.push_str(&format!(
                            "✔ Model set to: \x1b[1;32m{}\x1b[0m\r\n\r\n{}",
                            cfg.model, PROMPT_LABEL
                        ));
                    }
                }
            }
            Some("key") => {
                if let Some(val) = parts.get(2) {
                    cfg.api_key = Some(val.to_string());
                    cfg.save();
                    if let Ok(mut sb) = self.scrollback.lock() {
                        sb.push_str(&format!("✔ API key saved.\r\n\r\n{}", PROMPT_LABEL));
                    }
                }
            }
            _ => {
                if let Ok(mut sb) = self.scrollback.lock() {
                    sb.push_str(
                        "Usage: /config [endpoint <url> | model <name> | key <api-key>]\r\n\r\n",
                    );
                    sb.push_str(PROMPT_LABEL);
                }
            }
        }
    }

    fn run_model_detection(&self) {
        if let Ok(mut sb) = self.scrollback.lock() {
            sb.push_str(
                "Scanning localhost for local LLM engines (Ollama, LM Studio, vLLM, etc.)...\r\n",
            );
        }

        let scrollback = self.scrollback.clone();
        let menu_holder = self.active_menu.clone();
        let config = self.config.lock().unwrap().clone();

        wasm_bindgen_futures::spawn_local(async move {
            let discovered = probe::discover_local_models(|msg| {
                if let Ok(mut sb) = scrollback.lock() {
                    sb.push_str(msg);
                }
            })
            .await;

            if discovered.is_empty() {
                if let Ok(mut sb) = scrollback.lock() {
                    sb.push_str("\r\n\x1b[33m⚠ No local LLM servers responded on localhost:11434, 1234, 8000, 8080.\x1b[0m\r\n");
                    sb.push_str("  \x1b[2mMake sure Ollama (with OLLAMA_ORIGINS=\"*\") or LM Studio (with CORS enabled) is running.\x1b[0m\r\n");
                    sb.push_str("  \x1b[2mOr configure an endpoint manually via `/config endpoint <url>`.\x1b[0m\r\n\r\n");
                    sb.push_str(PROMPT_LABEL);
                }
            } else {
                let mut menu = ModelMenu::new(discovered, &config);
                let initial_render = menu.render_initial(&config);
                *menu_holder.lock().unwrap() = Some(menu);

                if let Ok(mut sb) = scrollback.lock() {
                    sb.push_str(&initial_render);
                }
            }
        });
    }

    fn run_query(&self, prompt: String) {
        *self.is_busy.lock().unwrap() = true;

        let cfg = self.config.lock().unwrap().clone();
        let mcp = self.mcp.clone();
        let waker = self.waker.clone();
        let scrollback = self.scrollback.clone();
        let messages_holder = self.messages.clone();
        let is_busy = self.is_busy.clone();

        {
            let mut msgs = messages_holder.lock().unwrap();
            msgs.push(json!({
                "role": "user",
                "content": prompt,
            }));
        }

        wasm_bindgen_futures::spawn_local(async move {
            let mut msgs = messages_holder.lock().unwrap().clone();

            let sb_cb = scrollback.clone();
            let on_output = move |token: &str| {
                if let Ok(mut sb) = sb_cb.lock() {
                    sb.push_str(token);
                }
            };

            let res = client::run_agent_loop(cfg, mcp, waker, &mut msgs, on_output).await;

            *messages_holder.lock().unwrap() = msgs;
            *is_busy.lock().unwrap() = false;

            if let Ok(mut sb) = scrollback.lock() {
                if let Err(err) = res {
                    sb.push_str(&format!("\r\n\x1b[31m[Error: {err}]\x1b[0m\r\n"));
                }
                sb.push_str(&format!("\r\n{}", PROMPT_LABEL));
            }
        });
    }
}

/// Writer adapter forwarding raw keyboard events directly to the `Harness`.
pub struct HarnessWriter {
    harness: Arc<Harness>,
}

impl Write for HarnessWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.harness.handle_bytes(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Spawns an interactive `TerminalSession` connected to an in-process `Harness`.
pub fn create_harness_session(
    id: String,
    mcp: InProcessMcp,
    waker: EventLoopProxy<()>,
) -> (TerminalSession, Arc<Harness>) {
    let scrollback = Arc::new(Mutex::new(Scrollback::default()));
    let harness = Arc::new(Harness::new(mcp, waker, scrollback.clone()));
    harness.init_banner();

    let writer = Box::new(HarnessWriter {
        harness: harness.clone(),
    });
    let session = TerminalSession::with_writer(id, scrollback, writer);

    (session, harness)
}
