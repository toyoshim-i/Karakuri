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

use super::ansi::{Cell, Scrollback};

/// Interactive terminal session running inside a native PTY.
pub struct TerminalSession {
    /// Unique identifier for this session.
    pub id: String,
    /// Lifecycle status of this session.
    pub status: Arc<Mutex<SessionStatus>>,
    /// Scrollback line buffer.
    pub scrollback: Arc<Mutex<Scrollback>>,
    writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
    master: Arc<Mutex<Option<Box<dyn portable_pty::MasterPty + Send>>>>,
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
        let master = Arc::new(Mutex::new(None));
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
                    sb.push_str(&err_msg);
                }
                if let Ok(mut st) = status.lock() {
                    *st = SessionStatus::Failed(err_msg);
                }
                return Self {
                    id,
                    status,
                    scrollback,
                    writer,
                    master,
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
                    sb.push_str(&err_msg);
                }
                if let Ok(mut st) = status.lock() {
                    *st = SessionStatus::Failed(err_msg);
                }
                return Self {
                    id,
                    status,
                    scrollback,
                    writer,
                    master,
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
                    master,
                    child,
                };
            }
        };

        if let Ok(w) = pty_pair.master.take_writer() {
            if let Ok(mut w_lock) = writer.lock() {
                *w_lock = Some(w);
            }
        }

        if let Ok(mut m_lock) = master.lock() {
            *m_lock = Some(pty_pair.master);
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
                while let Ok(n) = reader.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    let chunk = String::from_utf8_lossy(&buf[..n]);
                    if let Ok(mut sb) = scrollback_clone.lock() {
                        for c in chunk.chars() {
                            sb.push_char(c);
                        }
                    }
                }

                // Check child exit status
                let exit_code = if let Ok(mut c_lock) = child_clone.lock() {
                    if let Some(ref mut c) = *c_lock {
                        c.wait().ok().map(|s| s.exit_code())
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Ok(mut st) = status_clone.lock() {
                    *st = SessionStatus::Exited(exit_code);
                }
            })
            .ok();

        Self {
            id,
            status,
            scrollback,
            writer,
            master,
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

    /// Returns the current lifecycle status of this session, actively polling child exit status.
    pub fn status(&self) -> SessionStatus {
        let is_running = self
            .status
            .lock()
            .map(|st| matches!(*st, SessionStatus::Running))
            .unwrap_or(false);

        if is_running {
            if let Ok(mut c_lock) = self.child.lock() {
                if let Some(ref mut c) = *c_lock {
                    match c.try_wait() {
                        Ok(Some(exit_status)) => {
                            let code = exit_status.exit_code();
                            if let Ok(mut st) = self.status.lock() {
                                *st = SessionStatus::Exited(Some(code));
                            }
                            if let Ok(mut w_lock) = self.writer.lock() {
                                *w_lock = None;
                            }
                        }
                        Err(_) => {
                            if let Ok(mut st) = self.status.lock() {
                                *st = SessionStatus::Exited(None);
                            }
                            if let Ok(mut w_lock) = self.writer.lock() {
                                *w_lock = None;
                            }
                        }
                        Ok(None) => {}
                    }
                }
            }
        }

        self.status
            .lock()
            .map(|st| st.clone())
            .unwrap_or(SessionStatus::Exited(None))
    }

    /// Returns whether the child process is still actively running.
    pub fn is_running(&self) -> bool {
        matches!(self.status(), SessionStatus::Running)
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

    /// Returns a snapshot of all rendered character cell rows.
    pub fn rows(&self) -> Vec<Vec<Cell>> {
        self.scrollback
            .lock()
            .map(|sb| sb.rows.clone())
            .unwrap_or_default()
    }

    /// Returns whether the terminal cursor is currently set to visible.
    pub fn is_cursor_visible(&self) -> bool {
        self.scrollback
            .lock()
            .map(|sb| sb.cursor_visible)
            .unwrap_or(true)
    }

    /// Resizes the PTY terminal window dimensions.
    pub fn resize(&self, rows: u16, cols: u16) {
        if let Ok(mut m_lock) = self.master.lock() {
            if let Some(ref mut m) = *m_lock {
                let _ = m.resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                });
            }
        }
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
                let trimmed = if cmd.trim().is_empty() {
                    super::cli::default_custom_command()
                } else {
                    cmd.trim()
                };
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
                let trimmed = if cmd.trim().is_empty() {
                    super::cli::default_custom_command()
                } else {
                    cmd.trim()
                };
                format!("custom:{trimmed}")
            }
        };
        let lock = self.sessions.lock().ok()?;
        lock.get(&id).cloned()
    }

    /// Removes and cleans up the session for the given selection.
    pub fn remove(&self, selection: &CliSelection) {
        let id = match selection {
            CliSelection::Unselected => return,
            CliSelection::Preset(preset) => preset.command().to_owned(),
            CliSelection::Custom(cmd) => {
                let trimmed = if cmd.trim().is_empty() {
                    super::cli::default_custom_command()
                } else {
                    cmd.trim()
                };
                format!("custom:{trimmed}")
            }
        };
        if let Ok(mut lock) = self.sessions.lock() {
            lock.remove(&id);
        }
    }

    /// Resets / removes the session for the given selection so it can be restarted.
    pub fn restart(&self, selection: &CliSelection) -> Option<Arc<TerminalSession>> {
        self.remove(selection);
        self.get_or_spawn(selection)
    }
}
