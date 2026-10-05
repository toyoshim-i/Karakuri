//! In-Process Agent Harness orchestrator for WebAssembly.
//! Provides terminal prompt interaction, localhost model detection,
//! slash command autocompletion, clipboard export, and streaming MCP tool calling.

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

const PROMPT_LABEL: &str = "\x1b[1;36mYou\x1b[0m > ";
const AGENT_LABEL: &str = "\x1b[1;35mKarakuri\x1b[0m > ";

const SYSTEM_PROMPT: &str = r#"You are the Karakuri AI VJ Assistant running directly inside the browser.
You control a real-time procedural WebGPU visual performance engine with 14 in-process MCP tools.
You can read slots, wire parameters, transition decks, inspect performance costs, and execute operations.
When the user asks you to operate the mixer, change BPM, swap procedures, or alter visuals, call the appropriate Karakuri tool.
Respond concisely and helpfully in the user's language."#;

#[derive(Debug, Clone, Copy)]
pub struct CommandDef {
    pub name: &'static str,
    pub args_hint: &'static str,
    pub description: &'static str,
}

pub const COMMANDS: &[CommandDef] = &[
    CommandDef {
        name: "/model",
        args_hint: "",
        description: "Probe localhost and select a local LLM",
    },
    CommandDef {
        name: "/config",
        args_hint: "[endpoint|model|key]",
        description: "View or configure LLM endpoint & model",
    },
    CommandDef {
        name: "/copy",
        args_hint: "",
        description: "Copy all terminal text to clipboard",
    },
    CommandDef {
        name: "/clear",
        args_hint: "",
        description: "Clear terminal scrollback",
    },
    CommandDef {
        name: "/help",
        args_hint: "",
        description: "Show available commands help",
    },
    CommandDef {
        name: "/reset",
        args_hint: "",
        description: "Reset conversation history",
    },
];

pub struct Harness {
    config: Arc<Mutex<HarnessConfig>>,
    mcp: InProcessMcp,
    waker: EventLoopProxy<()>,
    scrollback: Arc<Mutex<Scrollback>>,
    messages: Arc<Mutex<Vec<serde_json::Value>>>,
    input_buffer: Arc<Mutex<String>>,
    active_menu: Arc<Mutex<Option<ModelMenu>>>,
    is_busy: Arc<Mutex<bool>>,
    completion_index: Arc<Mutex<Option<usize>>>,
}

/// Normalizes all standalone `\n` into `\r\n` to prevent staircase cursor indentation.
pub fn normalize_crlf(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let mut prev = '\0';
    for ch in s.chars() {
        if ch == '\n' && prev != '\r' {
            out.push('\r');
        }
        out.push(ch);
        prev = ch;
    }
    out
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
            completion_index: Arc::new(Mutex::new(None)),
        }
    }

    /// Prints the compact, responsive welcome banner and initial prompt.
    pub fn init_banner(&self) {
        let cfg = self.config.lock().unwrap().clone();
        let mut out = String::new();
        out.push_str("\r\n\x1b[1;36mKarakuri Web Agent Harness\x1b[0m\r\n");
        out.push_str(&format!(
            "  \x1b[2mModel:\x1b[0m \x1b[1;32m{}\x1b[0m \x1b[2m({})\x1b[0m\r\n",
            cfg.model, cfg.endpoint
        ));
        out.push_str(
            "  \x1b[2mType \x1b[1;32m/\x1b[0;2m for commands (/model, /help, etc.)\x1b[0m\r\n\r\n",
        );
        out.push_str(PROMPT_LABEL);

        if let Ok(mut sb) = self.scrollback.lock() {
            sb.push_str(&normalize_crlf(&out));
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

        // 3. Tab: Slash command autocompletion
        if bytes == b"\t" {
            self.handle_tab_completion();
            return;
        }

        // 4. Enter: Execute command or prompt
        if bytes == b"\r" || bytes == b"\n" {
            *self.completion_index.lock().unwrap() = None;
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
            return;
        }

        // 5. Backspace
        if bytes == b"\x7f" || bytes == b"\x08" {
            *self.completion_index.lock().unwrap() = None;
            let mut buf = self.input_buffer.lock().unwrap();
            if !buf.is_empty() {
                buf.pop();
                if let Ok(mut sb) = self.scrollback.lock() {
                    sb.push_str("\x08 \x08");
                }
            }
            return;
        }

        // 6. Ctrl+C
        if bytes == b"\x03" {
            *self.completion_index.lock().unwrap() = None;
            self.input_buffer.lock().unwrap().clear();
            if let Ok(mut sb) = self.scrollback.lock() {
                sb.push_str("^C\r\n");
                sb.push_str(PROMPT_LABEL);
            }
            return;
        }

        // 7. Ctrl+L (Clear screen)
        if bytes == b"\x0c" {
            *self.completion_index.lock().unwrap() = None;
            if let Ok(mut sb) = self.scrollback.lock() {
                sb.push_str("\x1b[2J\x1b[H");
                sb.push_str(PROMPT_LABEL);
                let buf = self.input_buffer.lock().unwrap();
                sb.push_str(&buf);
            }
            return;
        }

        // 8. Normal text input (including multibyte Japanese IME text)
        if let Ok(s) = std::str::from_utf8(bytes) {
            if !s.chars().any(|c| c.is_control()) {
                *self.completion_index.lock().unwrap() = None;
                self.input_buffer.lock().unwrap().push_str(s);
                if let Ok(mut sb) = self.scrollback.lock() {
                    sb.push_str(s);
                }

                // If user just typed '/', hint available commands inline
                if self.input_buffer.lock().unwrap().as_str() == "/" {
                    self.show_command_hints();
                }
            }
        }
    }

    /// Handles Tab key pressing for cycling / completing slash commands.
    fn handle_tab_completion(&self) {
        let current = self.input_buffer.lock().unwrap().clone();
        if !current.starts_with('/') {
            return;
        }

        let matches: Vec<&CommandDef> = COMMANDS
            .iter()
            .filter(|cmd| cmd.name.starts_with(&current))
            .collect();

        if matches.is_empty() {
            return;
        }

        let mut idx_lock = self.completion_index.lock().unwrap();
        let next_idx = match *idx_lock {
            Some(i) => (i + 1) % matches.len(),
            None => 0,
        };
        *idx_lock = Some(next_idx);

        let chosen = matches[next_idx];
        let replacement = format!("{} ", chosen.name);

        // Erase old input from terminal line
        let old_len = current.len();
        let mut erase_seq = String::new();
        for _ in 0..old_len {
            erase_seq.push_str("\x08 \x08");
        }
        erase_seq.push_str(&replacement);

        *self.input_buffer.lock().unwrap() = replacement;

        if let Ok(mut sb) = self.scrollback.lock() {
            sb.push_str(&erase_seq);
        }
    }

    /// Shows an informative hint list when user enters `/`.
    fn show_command_hints(&self) {
        let mut hint = String::new();
        hint.push_str("\r\n\x1b[2m[Commands: ");
        for (i, cmd) in COMMANDS.iter().enumerate() {
            if i > 0 {
                hint.push_str(" · ");
            }
            hint.push_str(cmd.name);
        }
        hint.push_str(" (Tab to complete)]\x1b[0m\r\n");
        hint.push_str(PROMPT_LABEL);
        hint.push('/');

        if let Ok(mut sb) = self.scrollback.lock() {
            sb.push_str(&normalize_crlf(&hint));
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
        } else if line == "/copy" {
            self.copy_output_to_clipboard();
        } else if line == "/help" {
            if let Ok(mut sb) = self.scrollback.lock() {
                let mut out = String::new();
                out.push_str("\x1b[1;36mAvailable Commands:\x1b[0m\r\n");
                for cmd in COMMANDS {
                    let hint = if cmd.args_hint.is_empty() {
                        cmd.name.to_string()
                    } else {
                        format!("{} {}", cmd.name, cmd.args_hint)
                    };
                    out.push_str(&format!(
                        "  \x1b[1;32m{:<24}\x1b[0m {}\r\n",
                        hint, cmd.description
                    ));
                }
                out.push_str("\r\n");
                out.push_str(PROMPT_LABEL);
                sb.push_str(&normalize_crlf(&out));
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

    /// Copies the current scrollback text to the browser clipboard via Navigator API.
    fn copy_output_to_clipboard(&self) {
        let lines = if let Ok(sb) = self.scrollback.lock() {
            sb.lines()
        } else {
            Vec::new()
        };

        let full_text = lines.join("\n");
        let line_count = lines.len();

        let scrollback = self.scrollback.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let res = if let Some(win) = web_sys::window() {
                let clip = win.navigator().clipboard();
                let promise = clip.write_text(&full_text);
                wasm_bindgen_futures::JsFuture::from(promise).await
            } else {
                Err(wasm_bindgen::JsValue::from_str("no window object"))
            };

            if let Ok(mut sb) = scrollback.lock() {
                match res {
                    Ok(_) => {
                        sb.push_str(&format!(
                            "✔ Copied terminal output ({} lines) to clipboard.\r\n\r\n{}",
                            line_count, PROMPT_LABEL
                        ));
                    }
                    Err(e) => {
                        sb.push_str(&format!(
                            "\x1b[31m✖ Failed to copy to clipboard: {:?}\x1b[0m\r\n\r\n{}",
                            e, PROMPT_LABEL
                        ));
                    }
                }
            }
        });
    }

    fn handle_config_command(&self, line: &str) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut cfg = self.config.lock().unwrap();

        if parts.len() == 1 {
            if let Ok(mut sb) = self.scrollback.lock() {
                let msg = format!(
                    "Current Configuration:\r\n  Endpoint: \x1b[33m{}\x1b[0m\r\n  Model:    \x1b[1;32m{}\x1b[0m\r\n  Key:      \x1b[2m{}\x1b[0m\r\n\r\n{}",
                    cfg.endpoint,
                    cfg.model,
                    if cfg.api_key.is_some() { "[set]" } else { "[none]" },
                    PROMPT_LABEL
                );
                sb.push_str(&normalize_crlf(&msg));
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
                    sb.push_str(&normalize_crlf(msg));
                }
            })
            .await;

            if discovered.is_empty() {
                if let Ok(mut sb) = scrollback.lock() {
                    let mut out = String::new();
                    out.push_str("\r\n\x1b[33m⚠ No local LLM servers responded on localhost:11434, 1234, 8000, 8080.\x1b[0m\r\n");
                    out.push_str("  \x1b[2mMake sure Ollama (with OLLAMA_ORIGINS=\"*\") or LM Studio (with CORS enabled) is running.\x1b[0m\r\n");
                    out.push_str("  \x1b[2mOr configure an endpoint manually via `/config endpoint <url>`.\x1b[0m\r\n\r\n");
                    out.push_str(PROMPT_LABEL);
                    sb.push_str(&normalize_crlf(&out));
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
            let agent_started = Arc::new(Mutex::new(false));
            let agent_started_cb = agent_started.clone();

            let on_output = move |token: &str| {
                if let Ok(mut sb) = sb_cb.lock() {
                    let mut started = agent_started_cb.lock().unwrap();
                    if !*started {
                        sb.push_str(AGENT_LABEL);
                        *started = true;
                    }
                    sb.push_str(&normalize_crlf(token));
                }
            };

            let res = client::run_agent_loop(cfg, mcp, waker, &mut msgs, on_output).await;

            *messages_holder.lock().unwrap() = msgs;
            *is_busy.lock().unwrap() = false;

            if let Ok(mut sb) = scrollback.lock() {
                if let Err(err) = res {
                    sb.push_str(&normalize_crlf(&format!(
                        "\r\n\x1b[31m[Error: {err}]\x1b[0m\r\n"
                    )));
                }
                sb.push_str(&format!("\r\n\r\n{}", PROMPT_LABEL));
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
