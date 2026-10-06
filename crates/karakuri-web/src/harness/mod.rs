//! In-Process Agent Harness orchestrator for WebAssembly.
//! Provides terminal prompt interaction, localhost model detection,
//! slash command autocompletion, clipboard export, and streaming MCP tool calling.

pub mod client;
pub mod config;
pub mod editor;
pub mod menu;
pub mod probe;
pub mod tools;

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
use self::editor::{
    insert_str_at, next_word_boundary, prev_word_boundary, redraw_prompt_line, remove_char_at,
    remove_char_range,
};
use self::menu::{MenuAction, ModelMenu};

pub const PROMPT_LABEL: &str = "\x1b[1;36mYou\x1b[0m > ";
const AGENT_LABEL: &str = "\x1b[1;35mKarakuri\x1b[0m > ";

const SYSTEM_PROMPT: &str = r#"You are the Karakuri AI VJ Assistant running directly inside the browser.
You control a real-time procedural WebGPU visual performance engine with in-process MCP tools.
You can read slots, wire parameters, transition decks, inspect performance costs, and execute operations.
When the user asks you to operate the mixer, change BPM, swap procedures, or alter visuals, invoke the appropriate function/tool call directly. Do not output raw JSON tool call blocks in your text message unless asked.
Respond concisely and helpfully in the user's language."#;

#[derive(Debug, Clone, Copy)]
pub struct CommandDef {
    pub name: &'static str,
    pub args_hint: &'static str,
    pub description: &'static str,
}

pub const COMMANDS: &[CommandDef] = &[
    CommandDef {
        name: "/tools",
        args_hint: "[tool_name]",
        description: "List available WebMCP tools or inspect schema",
    },
    CommandDef {
        name: "/call",
        args_hint: "<tool_name> [args]",
        description: "Directly execute an MCP tool with JSON or key=val args",
    },
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
    cursor_pos: Arc<Mutex<usize>>,
    undo_stack: Arc<Mutex<Vec<(String, usize)>>>,
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
            cursor_pos: Arc::new(Mutex::new(0)),
            undo_stack: Arc::new(Mutex::new(Vec::new())),
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
            "  \x1b[2mType \x1b[1;32m/\x1b[0;2m for commands (/tools, /call, /model, /help)\x1b[0m\r\n\r\n",
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

        let mut buf_lock = self.input_buffer.lock().unwrap();
        let mut cur_lock = self.cursor_pos.lock().unwrap();
        let mut undo_lock = self.undo_stack.lock().unwrap();

        let save_undo = |buf: &String, cur: usize, undo: &mut Vec<(String, usize)>| {
            if undo.last().map(|(s, _)| s != buf).unwrap_or(true) {
                undo.push((buf.clone(), cur));
                if undo.len() > 50 {
                    undo.remove(0);
                }
            }
        };

        // 3. Tab: Slash command autocompletion
        if bytes == b"\t" {
            drop(buf_lock);
            drop(cur_lock);
            drop(undo_lock);
            self.handle_tab_completion();
            return;
        }

        // 4. Enter: Execute command or prompt
        if bytes == b"\r" || bytes == b"\n" {
            *self.completion_index.lock().unwrap() = None;
            let line = buf_lock.clone();
            buf_lock.clear();
            *cur_lock = 0;
            undo_lock.clear();
            drop(buf_lock);
            drop(cur_lock);
            drop(undo_lock);

            if let Ok(mut sb) = self.scrollback.lock() {
                sb.push_str("\r\n");
            }
            self.execute_line(&line);
            return;
        }

        // 5. Backspace: delete character before cursor
        if bytes == b"\x7f" || bytes == b"\x08" {
            *self.completion_index.lock().unwrap() = None;
            if *cur_lock > 0 {
                save_undo(&buf_lock, *cur_lock, &mut undo_lock);
                *cur_lock -= 1;
                remove_char_at(&mut buf_lock, *cur_lock);
                if let Ok(mut sb) = self.scrollback.lock() {
                    redraw_prompt_line(&mut sb, &buf_lock, *cur_lock);
                }
            }
            return;
        }

        // 6. Delete character at cursor (Delete key \x1b[3~ or Ctrl+D \x04)
        if bytes == b"\x1b[3~" || bytes == b"\x04" {
            *self.completion_index.lock().unwrap() = None;
            let total = buf_lock.chars().count();
            if *cur_lock < total {
                save_undo(&buf_lock, *cur_lock, &mut undo_lock);
                remove_char_at(&mut buf_lock, *cur_lock);
                if let Ok(mut sb) = self.scrollback.lock() {
                    redraw_prompt_line(&mut sb, &buf_lock, *cur_lock);
                }
            }
            return;
        }

        // 7. Delete line to start (⌘Backspace / Ctrl+U \x15)
        if bytes == b"\x15" {
            *self.completion_index.lock().unwrap() = None;
            if *cur_lock > 0 {
                save_undo(&buf_lock, *cur_lock, &mut undo_lock);
                remove_char_range(&mut buf_lock, 0, *cur_lock);
                *cur_lock = 0;
                if let Ok(mut sb) = self.scrollback.lock() {
                    redraw_prompt_line(&mut sb, &buf_lock, *cur_lock);
                }
            }
            return;
        }

        // 8. Delete word before cursor (⌥Backspace / Ctrl+Backspace / Ctrl+W \x17)
        if bytes == b"\x17" {
            *self.completion_index.lock().unwrap() = None;
            if *cur_lock > 0 {
                save_undo(&buf_lock, *cur_lock, &mut undo_lock);
                let prev = prev_word_boundary(&buf_lock, *cur_lock);
                remove_char_range(&mut buf_lock, prev, *cur_lock);
                *cur_lock = prev;
                if let Ok(mut sb) = self.scrollback.lock() {
                    redraw_prompt_line(&mut sb, &buf_lock, *cur_lock);
                }
            }
            return;
        }

        // 9. Delete word after cursor (⌥Delete / Ctrl+Delete \x1bd)
        if bytes == b"\x1bd" {
            *self.completion_index.lock().unwrap() = None;
            let total = buf_lock.chars().count();
            if *cur_lock < total {
                save_undo(&buf_lock, *cur_lock, &mut undo_lock);
                let next = next_word_boundary(&buf_lock, *cur_lock);
                remove_char_range(&mut buf_lock, *cur_lock, next);
                if let Ok(mut sb) = self.scrollback.lock() {
                    redraw_prompt_line(&mut sb, &buf_lock, *cur_lock);
                }
            }
            return;
        }

        // 10. Delete line to end (⌘K / Ctrl+K \x0b)
        if bytes == b"\x0b" {
            *self.completion_index.lock().unwrap() = None;
            let total = buf_lock.chars().count();
            if *cur_lock < total {
                save_undo(&buf_lock, *cur_lock, &mut undo_lock);
                remove_char_range(&mut buf_lock, *cur_lock, total);
                if let Ok(mut sb) = self.scrollback.lock() {
                    redraw_prompt_line(&mut sb, &buf_lock, *cur_lock);
                }
            }
            return;
        }

        // 11. Cursor to start of line (⌘Left / Home / Ctrl+A: \x1b[H or \x01)
        if bytes == b"\x1b[H" || bytes == b"\x01" {
            *cur_lock = 0;
            if let Ok(mut sb) = self.scrollback.lock() {
                redraw_prompt_line(&mut sb, &buf_lock, *cur_lock);
            }
            return;
        }

        // 12. Cursor to end of line (⌘Right / End / Ctrl+E: \x1b[F or \x05)
        if bytes == b"\x1b[F" || bytes == b"\x05" {
            *cur_lock = buf_lock.chars().count();
            if let Ok(mut sb) = self.scrollback.lock() {
                redraw_prompt_line(&mut sb, &buf_lock, *cur_lock);
            }
            return;
        }

        // 13. Cursor one character left (ArrowLeft / Ctrl+B: \x1b[D or \x02)
        if bytes == b"\x1b[D" || bytes == b"\x02" {
            if *cur_lock > 0 {
                *cur_lock -= 1;
                if let Ok(mut sb) = self.scrollback.lock() {
                    redraw_prompt_line(&mut sb, &buf_lock, *cur_lock);
                }
            }
            return;
        }

        // 14. Cursor one character right (ArrowRight / Ctrl+F: \x1b[C or \x06)
        if bytes == b"\x1b[C" || bytes == b"\x06" {
            let total = buf_lock.chars().count();
            if *cur_lock < total {
                *cur_lock += 1;
                if let Ok(mut sb) = self.scrollback.lock() {
                    redraw_prompt_line(&mut sb, &buf_lock, *cur_lock);
                }
            }
            return;
        }

        // 15. Cursor word backward (⌥Left / Ctrl+Left: \x1bb)
        if bytes == b"\x1bb" {
            *cur_lock = prev_word_boundary(&buf_lock, *cur_lock);
            if let Ok(mut sb) = self.scrollback.lock() {
                redraw_prompt_line(&mut sb, &buf_lock, *cur_lock);
            }
            return;
        }

        // 16. Cursor word forward (⌥Right / Ctrl+Right: \x1bf)
        if bytes == b"\x1bf" {
            *cur_lock = next_word_boundary(&buf_lock, *cur_lock);
            if let Ok(mut sb) = self.scrollback.lock() {
                redraw_prompt_line(&mut sb, &buf_lock, *cur_lock);
            }
            return;
        }

        // 17. Undo (⌘Z / Ctrl+Z: \x1f or \x1a)
        if bytes == b"\x1f" || bytes == b"\x1a" {
            if let Some((prev_buf, prev_cur)) = undo_lock.pop() {
                *buf_lock = prev_buf;
                *cur_lock = prev_cur;
                if let Ok(mut sb) = self.scrollback.lock() {
                    redraw_prompt_line(&mut sb, &buf_lock, *cur_lock);
                }
            }
            return;
        }

        // 18. Ctrl+C: Cancel / clear current input line
        if bytes == b"\x03" {
            *self.completion_index.lock().unwrap() = None;
            buf_lock.clear();
            *cur_lock = 0;
            undo_lock.clear();
            if let Ok(mut sb) = self.scrollback.lock() {
                sb.push_str("^C\r\n");
                sb.push_str(PROMPT_LABEL);
            }
            return;
        }

        // 19. Ctrl+L: Clear screen
        if bytes == b"\x0c" {
            *self.completion_index.lock().unwrap() = None;
            if let Ok(mut sb) = self.scrollback.lock() {
                sb.push_str("\x1b[2J\x1b[H");
                redraw_prompt_line(&mut sb, &buf_lock, *cur_lock);
            }
            return;
        }

        // 20. Normal text input (including paste & multibyte Japanese IME text)
        if let Ok(s) = std::str::from_utf8(bytes) {
            if !s.chars().any(|c| c.is_control()) {
                *self.completion_index.lock().unwrap() = None;
                save_undo(&buf_lock, *cur_lock, &mut undo_lock);
                insert_str_at(&mut buf_lock, *cur_lock, s);
                *cur_lock += s.chars().count();
                if let Ok(mut sb) = self.scrollback.lock() {
                    redraw_prompt_line(&mut sb, &buf_lock, *cur_lock);
                }

                // If user just typed '/', hint available commands inline
                if buf_lock.as_str() == "/" {
                    drop(buf_lock);
                    drop(cur_lock);
                    drop(undo_lock);
                    self.show_command_hints();
                }
            }
        }
    }

    /// Handles Tab key pressing for cycling / completing slash commands and tool names.
    fn handle_tab_completion(&self) {
        let current = self.input_buffer.lock().unwrap().clone();
        if !current.starts_with('/') {
            return;
        }

        // Sub-command / tool name completion for /call and /tools
        let tool_subcmd = if let Some(query) = current.strip_prefix("/call ") {
            Some(("/call ", query))
        } else {
            current
                .strip_prefix("/tools ")
                .map(|query| ("/tools ", query))
        };

        if let Some((cmd_prefix, query)) = tool_subcmd {
            // Only complete if completing the tool name itself (no subsequent space)
            if !query.contains(' ') {
                let mcp_tools = self.mcp.tools();
                let tool_names = tools::get_tool_names(&mcp_tools);
                let matches: Vec<&String> = tool_names
                    .iter()
                    .filter(|name| name.starts_with(query))
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
                let replacement = format!("{cmd_prefix}{chosen} ");

                *self.input_buffer.lock().unwrap() = replacement.clone();
                *self.cursor_pos.lock().unwrap() = replacement.chars().count();

                if let Ok(mut sb) = self.scrollback.lock() {
                    redraw_prompt_line(&mut sb, &replacement, replacement.chars().count());
                }
                return;
            }
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

        *self.input_buffer.lock().unwrap() = replacement.clone();
        *self.cursor_pos.lock().unwrap() = replacement.chars().count();

        if let Ok(mut sb) = self.scrollback.lock() {
            redraw_prompt_line(&mut sb, &replacement, replacement.chars().count());
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

        if line == "/tools" || line.starts_with("/tools ") {
            self.handle_tools_command(line);
        } else if line == "/call" || line.starts_with("/call ") {
            self.handle_call_command(line);
        } else if line == "/model" {
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

    /// Handles the `/tools` command to list tools or inspect a specific tool.
    fn handle_tools_command(&self, line: &str) {
        let remainder = line.strip_prefix("/tools").unwrap_or("").trim();
        let mcp_tools = self.mcp.tools();

        if let Ok(mut sb) = self.scrollback.lock() {
            let output = if remainder.is_empty() {
                tools::format_tools_overview(&mcp_tools)
            } else {
                tools::format_tool_detail(&mcp_tools, remainder)
            };
            sb.push_str(&normalize_crlf(&output));
            sb.push_str("\r\n");
            sb.push_str(PROMPT_LABEL);
        }
    }

    /// Handles the `/call` command to directly invoke an MCP tool.
    fn handle_call_command(&self, line: &str) {
        let remainder = line.strip_prefix("/call").unwrap_or("").trim();
        if remainder.is_empty() {
            if let Ok(mut sb) = self.scrollback.lock() {
                let usage = tools::call_usage_help();
                sb.push_str(&normalize_crlf(&usage));
                sb.push_str("\r\n");
                sb.push_str(PROMPT_LABEL);
            }
            return;
        }

        let mut parts = remainder.splitn(2, char::is_whitespace);
        let tool_name = parts.next().unwrap_or("").trim();
        let args_str = parts.next().unwrap_or("").trim();

        let mcp_tools = self.mcp.tools();
        let tool_schema = tools::find_tool_schema(&mcp_tools, tool_name);

        match tools::parse_call_args(args_str, tool_schema) {
            Ok(args) => {
                self.run_mcp_call(tool_name.to_string(), args);
            }
            Err(err) => {
                if let Ok(mut sb) = self.scrollback.lock() {
                    sb.push_str(&normalize_crlf(&format!(
                        "\x1b[31m✖ Argument error:\x1b[0m {err}\r\n\r\n"
                    )));
                    sb.push_str(PROMPT_LABEL);
                }
            }
        }
    }

    /// Executes an MCP tool asynchronously against the engine and prints formatted output.
    fn run_mcp_call(&self, tool_name: String, args: serde_json::Value) {
        *self.is_busy.lock().unwrap() = true;

        if let Ok(mut sb) = self.scrollback.lock() {
            let pretty_args = if args.is_object() && args.as_object().is_some_and(|o| !o.is_empty())
            {
                format!(" with {args}")
            } else {
                String::new()
            };
            sb.push_str(&format!(
                "\x1b[36m⚡ Calling \x1b[1;36m{}\x1b[0;36m{}...\x1b[0m\r\n",
                tool_name, pretty_args
            ));
        }

        let mcp = self.mcp.clone();
        let waker = self.waker.clone();
        let scrollback = self.scrollback.clone();
        let is_busy = self.is_busy.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let res = crate::webmcp::execute_tool_async(mcp, waker, tool_name.clone(), args).await;
            *is_busy.lock().unwrap() = false;

            if let Ok(mut sb) = scrollback.lock() {
                let formatted = tools::format_call_result(&tool_name, res);
                sb.push_str(&normalize_crlf(&formatted));
                sb.push_str(&format!("\r\n\r\n{}", PROMPT_LABEL));
            }
        });
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
