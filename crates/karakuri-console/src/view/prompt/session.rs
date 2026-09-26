//! Background PTY terminal session execution and management for the Prompt bay.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use portable_pty::{native_pty_system, CommandBuilder, PtySize};

use super::cli::CliSelection;

/// Lifecycle status of an agent CLI terminal session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatus {
    /// Process is currently active and running.
    Running,
    /// Process has terminated with an exit code (if available).
    Exited(Option<u32>),
    /// Process failed to spawn or encountered an error.
    Failed(String),
}

/// ANSI terminal escape events for cursor manipulation, line clearing, and character output.
#[derive(Debug, PartialEq, Eq)]
pub enum AnsiEvent {
    Print(char),
    Newline,
    CarriageReturn,
    Backspace,
    Tab,
    ClearLine(u8),
    ClearDisplay(u8),
    CursorUp(usize),
    CursorDown(usize),
    CursorForward(usize),
    CursorBack(usize),
    CursorCol(usize),
    CursorPos(usize, usize),
}

#[derive(Default, PartialEq, Eq)]
enum AnsiState {
    #[default]
    Normal,
    Escape,
    Csi,
    Osc,
    Charset,
}

/// Streaming parser for ANSI escape sequences converting characters into terminal events.
#[derive(Default)]
struct AnsiParser {
    state: AnsiState,
    csi_params: String,
}

impl AnsiParser {
    fn parse_char(&mut self, c: char) -> Option<AnsiEvent> {
        match self.state {
            AnsiState::Normal => {
                if c == '\x1b' {
                    self.state = AnsiState::Escape;
                    None
                } else if c == '\r' {
                    Some(AnsiEvent::CarriageReturn)
                } else if c == '\n' {
                    Some(AnsiEvent::Newline)
                } else if c == '\x08' {
                    Some(AnsiEvent::Backspace)
                } else if c == '\t' {
                    Some(AnsiEvent::Tab)
                } else if c.is_control() {
                    None
                } else {
                    Some(AnsiEvent::Print(c))
                }
            }
            AnsiState::Escape => match c {
                '[' => {
                    self.state = AnsiState::Csi;
                    self.csi_params.clear();
                    None
                }
                ']' => {
                    self.state = AnsiState::Osc;
                    None
                }
                '(' | ')' | '*' | '+' => {
                    self.state = AnsiState::Charset;
                    None
                }
                _ => {
                    self.state = AnsiState::Normal;
                    None
                }
            },
            AnsiState::Csi => {
                if c.is_ascii_digit() || c == ';' || c == '?' || c == '<' || c == '>' {
                    self.csi_params.push(c);
                    None
                } else if ('@'..='~').contains(&c) {
                    self.state = AnsiState::Normal;
                    match c {
                        'K' => {
                            let n: u8 = self.csi_params.parse().unwrap_or(0);
                            Some(AnsiEvent::ClearLine(n))
                        }
                        'J' => {
                            let n: u8 = self.csi_params.parse().unwrap_or(0);
                            Some(AnsiEvent::ClearDisplay(n))
                        }
                        'A' => {
                            let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                            Some(AnsiEvent::CursorUp(n))
                        }
                        'B' => {
                            let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                            Some(AnsiEvent::CursorDown(n))
                        }
                        'C' => {
                            let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                            Some(AnsiEvent::CursorForward(n))
                        }
                        'D' => {
                            let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                            Some(AnsiEvent::CursorBack(n))
                        }
                        'G' => {
                            let n: usize = self.csi_params.parse().unwrap_or(1).max(1);
                            Some(AnsiEvent::CursorCol(n))
                        }
                        'H' | 'f' => {
                            let mut parts = self.csi_params.split(';');
                            let r: usize = parts
                                .next()
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(1)
                                .max(1);
                            let col: usize = parts
                                .next()
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(1)
                                .max(1);
                            Some(AnsiEvent::CursorPos(r, col))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            AnsiState::Osc => {
                if c == '\x07' || c == '\x1b' {
                    self.state = AnsiState::Normal;
                }
                None
            }
            AnsiState::Charset => {
                self.state = AnsiState::Normal;
                None
            }
        }
    }
}

/// Buffer holding terminal lines and active cursor position.
#[derive(Debug, Default)]
pub struct Scrollback {
    /// Rendered lines of text.
    pub lines: Vec<String>,
    /// Active cursor coordinates (row, col) (0-indexed).
    pub cursor: (usize, usize),
}

impl Scrollback {
    /// Maximum number of scrollback lines retained in memory.
    pub const MAX_LINES: usize = 2048;

    /// Dispatches an ANSI terminal event to update buffer lines and cursor.
    pub fn handle_event(&mut self, ev: AnsiEvent) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        match ev {
            AnsiEvent::Print(c) => {
                while self.lines.len() <= self.cursor.0 {
                    self.lines.push(String::new());
                }
                let line = &mut self.lines[self.cursor.0];
                let char_count = line.chars().count();
                if self.cursor.1 >= char_count {
                    while line.chars().count() < self.cursor.1 {
                        line.push(' ');
                    }
                    line.push(c);
                } else {
                    let mut new_line = String::with_capacity(line.len());
                    for (i, existing_ch) in line.chars().enumerate() {
                        if i == self.cursor.1 {
                            new_line.push(c);
                        } else {
                            new_line.push(existing_ch);
                        }
                    }
                    *line = new_line;
                }
                self.cursor.1 += 1;
            }
            AnsiEvent::Newline => {
                self.cursor.0 += 1;
                self.cursor.1 = 0;
                while self.lines.len() <= self.cursor.0 {
                    self.lines.push(String::new());
                }
                if self.lines.len() > Self::MAX_LINES {
                    self.lines.remove(0);
                    self.cursor.0 = self.cursor.0.saturating_sub(1);
                }
            }
            AnsiEvent::CarriageReturn => {
                self.cursor.1 = 0;
            }
            AnsiEvent::Backspace => {
                self.cursor.1 = self.cursor.1.saturating_sub(1);
            }
            AnsiEvent::Tab => {
                self.cursor.1 = (self.cursor.1 / 8 + 1) * 8;
            }
            AnsiEvent::ClearLine(mode) => {
                if self.cursor.0 < self.lines.len() {
                    match mode {
                        2 => {
                            self.lines[self.cursor.0].clear();
                            self.cursor.1 = 0;
                        }
                        1 => {
                            let line = &mut self.lines[self.cursor.0];
                            let remaining: String = line.chars().skip(self.cursor.1).collect();
                            *line = format!("{}{}", " ".repeat(self.cursor.1), remaining);
                        }
                        _ => {
                            let line = &mut self.lines[self.cursor.0];
                            let kept: String = line.chars().take(self.cursor.1).collect();
                            *line = kept;
                        }
                    }
                }
            }
            AnsiEvent::ClearDisplay(mode) => {
                if mode == 2 || mode == 3 {
                    self.lines.clear();
                    self.lines.push(String::new());
                    self.cursor = (0, 0);
                }
            }
            AnsiEvent::CursorUp(n) => {
                self.cursor.0 = self.cursor.0.saturating_sub(n);
            }
            AnsiEvent::CursorDown(n) => {
                self.cursor.0 = (self.cursor.0 + n).min(self.lines.len().saturating_sub(1));
            }
            AnsiEvent::CursorForward(n) => {
                self.cursor.1 += n;
            }
            AnsiEvent::CursorBack(n) => {
                self.cursor.1 = self.cursor.1.saturating_sub(n);
            }
            AnsiEvent::CursorCol(n) => {
                self.cursor.1 = n.saturating_sub(1);
            }
            AnsiEvent::CursorPos(r, col) => {
                self.cursor.0 = r.saturating_sub(1).min(self.lines.len().saturating_sub(1));
                self.cursor.1 = col.saturating_sub(1);
            }
        }
    }

    /// Pushes a single character into the scrollback buffer.
    pub fn push_char(&mut self, ch: char) {
        let mut parser = AnsiParser::default();
        if let Some(ev) = parser.parse_char(ch) {
            self.handle_event(ev);
        }
    }

    /// Returns all completed lines plus any current in-progress line.
    pub fn lines(&self) -> Vec<String> {
        self.lines.clone()
    }

    /// Returns the active cursor coordinates (row, col).
    pub fn cursor(&self) -> (usize, usize) {
        self.cursor
    }
}

/// Interactive terminal session running inside a native PTY.
pub struct TerminalSession {
    /// Unique identifier for this session.
    pub id: String,
    /// Lifecycle status of this session.
    pub status: Arc<Mutex<SessionStatus>>,
    /// Scrollback line buffer.
    pub scrollback: Arc<Mutex<Scrollback>>,
    writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
    child: Arc<Mutex<Option<Box<dyn portable_pty::Child + Send + Sync>>>>,
}

impl std::fmt::Debug for TerminalSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalSession")
            .field("id", &self.id)
            .field("status", &self.status)
            .finish()
    }
}

impl TerminalSession {
    /// Spawns a new interactive CLI session with a native PTY.
    pub fn spawn(id: String, program: &str, args: &[&str]) -> Self {
        let status = Arc::new(Mutex::new(SessionStatus::Running));
        let scrollback = Arc::new(Mutex::new(Scrollback::default()));
        let writer = Arc::new(Mutex::new(None));
        let child = Arc::new(Mutex::new(None));

        let pty_system = native_pty_system();
        let pty_pair = match pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(pair) => pair,
            Err(e) => {
                let err_msg = format!("[PTY allocation failed: {e}]");
                if let Ok(mut sb) = scrollback.lock() {
                    sb.lines.push(err_msg.clone());
                }
                if let Ok(mut st) = status.lock() {
                    *st = SessionStatus::Failed(err_msg);
                }
                return Self {
                    id,
                    status,
                    scrollback,
                    writer,
                    child,
                };
            }
        };

        let resolved_program = super::cli::resolve_executable(program)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| program.to_string());
        let mut cmd = CommandBuilder::new(&resolved_program);
        for arg in args {
            cmd.arg(arg);
        }
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        // Spawn child process in PTY slave
        let spawned_child = match pty_pair.slave.spawn_command(cmd) {
            Ok(child_proc) => child_proc,
            Err(e) => {
                let err_msg = format!("[Failed to start `{program}`: {e}]");
                if let Ok(mut sb) = scrollback.lock() {
                    sb.lines.push(err_msg.clone());
                }
                if let Ok(mut st) = status.lock() {
                    *st = SessionStatus::Failed(err_msg);
                }
                return Self {
                    id,
                    status,
                    scrollback,
                    writer,
                    child,
                };
            }
        };

        drop(pty_pair.slave);

        let reader = match pty_pair.master.try_clone_reader() {
            Ok(r) => r,
            Err(e) => {
                let err_msg = format!("[Failed to open PTY reader: {e}]");
                if let Ok(mut st) = status.lock() {
                    *st = SessionStatus::Failed(err_msg);
                }
                return Self {
                    id,
                    status,
                    scrollback,
                    writer,
                    child,
                };
            }
        };

        if let Ok(w) = pty_pair.master.take_writer() {
            if let Ok(mut w_lock) = writer.lock() {
                *w_lock = Some(w);
            }
        }

        if let Ok(mut c_lock) = child.lock() {
            *c_lock = Some(spawned_child);
        }

        // Spawn background reader thread
        let scrollback_clone = Arc::clone(&scrollback);
        let status_clone = Arc::clone(&status);
        let child_clone = Arc::clone(&child);
        let thread_id = id.clone();

        std::thread::Builder::new()
            .name(format!("pty-reader-{thread_id}"))
            .spawn(move || {
                let mut reader = reader;
                let mut buf = [0u8; 1024];
                let mut parser = AnsiParser::default();

                while let Ok(n) = reader.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    let chunk = String::from_utf8_lossy(&buf[..n]);
                    if let Ok(mut sb) = scrollback_clone.lock() {
                        for c in chunk.chars() {
                            if let Some(event) = parser.parse_char(c) {
                                sb.handle_event(event);
                            }
                        }
                    }
                }

                // Check child exit status
                if let Ok(mut c_lock) = child_clone.lock() {
                    if let Some(ref mut c) = *c_lock {
                        if let Ok(exit_status) = c.wait() {
                            if let Ok(mut st) = status_clone.lock() {
                                *st = SessionStatus::Exited(Some(exit_status.exit_code()));
                            }
                        }
                    }
                }
            })
            .ok();

        Self {
            id,
            status,
            scrollback,
            writer,
            child,
        }
    }

    /// Sends raw bytes directly to the PTY stdin.
    pub fn send_bytes(&self, bytes: &[u8]) -> std::io::Result<()> {
        let mut writer_lock = self
            .writer
            .lock()
            .map_err(|_| std::io::Error::other("session writer lock poisoned"))?;
        if let Some(ref mut w) = *writer_lock {
            w.write_all(bytes)?;
            w.flush()?;
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "session writer closed",
            ))
        }
    }

    /// Sends a line of text followed by carriage return (`\r`) to the PTY stdin.
    pub fn send_line(&self, text: &str) -> std::io::Result<()> {
        let mut writer_lock = self
            .writer
            .lock()
            .map_err(|_| std::io::Error::other("session writer lock poisoned"))?;
        if let Some(ref mut w) = *writer_lock {
            w.write_all(text.as_bytes())?;
            w.write_all(b"\r")?;
            w.flush()?;
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "session writer closed",
            ))
        }
    }

    /// Sends an interrupt signal (Ctrl+C / 0x03) to the session.
    pub fn send_interrupt(&self) -> std::io::Result<()> {
        let mut writer_lock = self
            .writer
            .lock()
            .map_err(|_| std::io::Error::other("session writer lock poisoned"))?;
        if let Some(ref mut w) = *writer_lock {
            w.write_all(&[0x03])?;
            w.flush()?;
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "session writer closed",
            ))
        }
    }

    /// Returns whether the child process is still actively running.
    pub fn is_running(&self) -> bool {
        self.status
            .lock()
            .map(|st| matches!(*st, SessionStatus::Running))
            .unwrap_or(false)
    }

    /// Returns a snapshot of all output lines from this session.
    pub fn lines(&self) -> Vec<String> {
        self.scrollback
            .lock()
            .map(|sb| sb.lines())
            .unwrap_or_default()
    }

    /// Returns the active cursor coordinates (row, col) in the terminal.
    pub fn cursor(&self) -> (usize, usize) {
        self.scrollback
            .lock()
            .map(|sb| sb.cursor())
            .unwrap_or((0, 0))
    }

    /// Returns the OS process ID if the child is still running.
    pub fn process_id(&self) -> Option<u32> {
        self.child
            .lock()
            .ok()
            .and_then(|c| c.as_ref().and_then(|child| child.process_id()))
    }
}

/// Manager maintaining active background CLI terminal sessions.
#[derive(Debug, Clone, Default)]
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, Arc<TerminalSession>>>>,
}

impl PartialEq for SessionManager {
    fn eq(&self, other: &Self) -> bool {
        let k1 = self
            .sessions
            .lock()
            .map(|s| s.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let k2 = other
            .sessions
            .lock()
            .map(|s| s.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        k1 == k2
    }
}

impl Eq for SessionManager {}

impl SessionManager {
    /// Creates an empty session manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets or spawns a session for the specified CLI selection.
    pub fn get_or_spawn(&self, selection: &CliSelection) -> Option<Arc<TerminalSession>> {
        let (id, program, args) = match selection {
            CliSelection::Unselected => return None,
            CliSelection::Preset(preset) => {
                let id = preset.command().to_owned();
                (id, preset.command().to_owned(), Vec::new())
            }
            CliSelection::Custom(cmd) => {
                let trimmed = cmd.trim();
                if trimmed.is_empty() {
                    return None;
                }
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                let (first, rest) = parts.split_first()?;
                let id = format!("custom:{trimmed}");
                let args: Vec<String> = rest.iter().map(|s| (*s).to_string()).collect();
                (id, (*first).to_string(), args)
            }
        };

        let mut lock = self.sessions.lock().ok()?;
        if let Some(existing) = lock.get(&id) {
            return Some(Arc::clone(existing));
        }

        let arg_slices: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let session = Arc::new(TerminalSession::spawn(id.clone(), &program, &arg_slices));
        lock.insert(id, Arc::clone(&session));
        Some(session)
    }

    /// Returns the currently active session if one exists for `selection`.
    pub fn active_session(&self, selection: &CliSelection) -> Option<Arc<TerminalSession>> {
        let id = match selection {
            CliSelection::Unselected => return None,
            CliSelection::Preset(preset) => preset.command().to_owned(),
            CliSelection::Custom(cmd) => {
                let trimmed = cmd.trim();
                if trimmed.is_empty() {
                    return None;
                }
                format!("custom:{trimmed}")
            }
        };
        let lock = self.sessions.lock().ok()?;
        lock.get(&id).cloned()
    }

    /// Resets / removes the session for the given selection so it can be restarted.
    pub fn restart(&self, selection: &CliSelection) -> Option<Arc<TerminalSession>> {
        let id = match selection {
            CliSelection::Unselected => return None,
            CliSelection::Preset(preset) => preset.command().to_owned(),
            CliSelection::Custom(cmd) => format!("custom:{}", cmd.trim()),
        };
        if let Ok(mut lock) = self.sessions.lock() {
            lock.remove(&id);
        }
        self.get_or_spawn(selection)
    }
}
