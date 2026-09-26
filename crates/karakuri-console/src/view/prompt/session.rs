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

/// Filter for ANSI escape sequences preserving UTF-8 text and newlines.
#[derive(Default)]
struct AnsiFilter {
    state: AnsiState,
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

impl AnsiFilter {
    fn filter_char(&mut self, c: char) -> Option<char> {
        match self.state {
            AnsiState::Normal => {
                if c == '\x1b' {
                    self.state = AnsiState::Escape;
                    None
                } else if c.is_control() && c != '\n' && c != '\r' && c != '\t' && c != '\x08' {
                    None
                } else {
                    Some(c)
                }
            }
            AnsiState::Escape => match c {
                '[' => {
                    self.state = AnsiState::Csi;
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
                if ('@'..='~').contains(&c) {
                    self.state = AnsiState::Normal;
                }
                None
            }
            AnsiState::Osc => {
                if c == '\x07' {
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

/// Buffer holding completed scrollback lines and in-progress line output.
#[derive(Debug, Default)]
pub struct Scrollback {
    /// Completed lines received from PTY.
    pub completed: Vec<String>,
    /// In-progress partial line currently being built.
    pub current: String,
    pending_cr: bool,
}

impl Scrollback {
    /// Maximum number of scrollback lines retained in memory.
    pub const MAX_LINES: usize = 2048;

    /// Pushes a single character into the scrollback buffer.
    pub fn push_char(&mut self, ch: char) {
        match ch {
            '\r' => {
                self.pending_cr = true;
            }
            '\n' => {
                self.pending_cr = false;
                let line = std::mem::take(&mut self.current);
                if self.completed.len() >= Self::MAX_LINES {
                    self.completed.remove(0);
                }
                self.completed.push(line);
            }
            '\x08' => {
                self.pending_cr = false;
                self.current.pop();
            }
            c => {
                if self.pending_cr {
                    self.pending_cr = false;
                    self.current.clear();
                }
                self.current.push(c);
            }
        }
    }

    /// Returns all completed lines plus any current in-progress line.
    pub fn lines(&self) -> Vec<String> {
        let mut all = self.completed.clone();
        if !self.current.is_empty() {
            all.push(self.current.clone());
        }
        all
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
                    sb.completed.push(err_msg.clone());
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

        let mut cmd = CommandBuilder::new(program);
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
                    sb.completed.push(err_msg.clone());
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
                let mut filter = AnsiFilter::default();

                while let Ok(n) = reader.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    let chunk = String::from_utf8_lossy(&buf[..n]);
                    if let Ok(mut sb) = scrollback_clone.lock() {
                        for c in chunk.chars() {
                            if let Some(filtered) = filter.filter_char(c) {
                                sb.push_char(filtered);
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

    /// Sends a line of text followed by newline to the PTY stdin.
    pub fn send_line(&self, text: &str) -> std::io::Result<()> {
        let mut writer_lock = self
            .writer
            .lock()
            .map_err(|_| std::io::Error::other("session writer lock poisoned"))?;
        if let Some(ref mut w) = *writer_lock {
            w.write_all(text.as_bytes())?;
            w.write_all(b"\n")?;
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
