//! Out-of-process output sink plugins (Syphon, Spout, NDI).
//! Spawns a foreign helper process, negotiates zero-copy GPU surface sharing, and transmits
//! frame notifications over a non-blocking pipe (ADR-0358, docs/plugins.md).

pub mod discovery;
pub mod message;

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub use discovery::{discover_plugins, probe_plugin, DiscoveredPlugin};
pub use message::{HostMessage, PluginMessage, PROTOCOL_VERSION};

const GREETING_TIMEOUT: Duration = Duration::from_secs(3);
const FRAME_QUEUE_CAPACITY: usize = 2;

/// Why an output plugin was refused during startup or negotiation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Incompatible wire protocol version.
    Version { theirs: u32, ours: u32 },
    /// Process exited or produced output before greeting with `Hello`.
    NoGreeting,
    /// Plugin does not support the requested zero-copy surface mechanism.
    UnsupportedSurface {
        wanted: String,
        offered: Vec<String>,
    },
    /// Plugin failed to acknowledge `Open` with `Ready`.
    NotReady(String),
    /// OS failure to spawn or communicate with child process.
    Process(String),
    /// Handshake timed out waiting for child reply.
    Timeout(String),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Version { theirs, ours } => write!(
                f,
                "plugin speaks protocol v{theirs} and host speaks v{ours} — update whichever is older"
            ),
            Refusal::NoGreeting => write!(f, "plugin closed pipe or sent message before Hello greeting"),
            Refusal::UnsupportedSurface { wanted, offered } => write!(
                f,
                "plugin does not support surface `{wanted}` (offered: {offered:?})"
            ),
            Refusal::NotReady(msg) => write!(f, "plugin failed to report ready: {msg}"),
            Refusal::Process(msg) => write!(f, "failed to spawn output plugin: {msg}"),
            Refusal::Timeout(msg) => write!(f, "timed out waiting for plugin: {msg}"),
        }
    }
}

impl std::error::Error for Refusal {}

/// Current live telemetry reported by the output plugin.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PluginTelemetry {
    /// Number of external receivers / clients currently attached (e.g. Resolume).
    pub clients: u32,
    /// Frames dropped due to plugin-side processing delays.
    pub plugin_dropped: u64,
    /// Frames dropped host-side due to pipe congestion (*the window never waits*).
    pub host_dropped: u64,
}

/// An active out-of-process output plugin session.
pub struct OutputPlugin {
    child: Child,
    name: String,
    server_name: String,
    tx: SyncSender<HostMessage>,
    host_dropped: Arc<AtomicU64>,
    connected_clients: Arc<AtomicU64>,
    plugin_dropped: Arc<AtomicU64>,
    alive: Arc<AtomicBool>,
}

impl OutputPlugin {
    /// Spawns `command`, negotiates protocol and surface type, and enters streaming state.
    pub fn open(
        command: &str,
        surface_kind: &str,
        width: u32,
        height: u32,
        format: &str,
    ) -> Result<Self, Refusal> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(Refusal::Process("empty command".into()));
        }

        let mut child = Command::new(parts[0])
            .args(&parts[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| Refusal::Process(format!("{}: {e}", parts[0])))?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| Refusal::Process("failed to capture child stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Refusal::Process("failed to capture child stdout".into()))?;

        let (msg_tx, msg_rx) = std::sync::mpsc::channel();
        let (err_tx, _err_rx) = std::sync::mpsc::channel();

        // Dedicated reader thread to avoid blocking on line reads
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        if let Ok(msg) = serde_json::from_str::<PluginMessage>(line) {
                            if msg_tx.send(msg).is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = err_tx.send(e);
                        break;
                    }
                }
            }
        });

        // 1. Await Hello greeting
        let hello = msg_rx
            .recv_timeout(GREETING_TIMEOUT)
            .map_err(|_| Refusal::Timeout("waiting for plugin Hello greeting".into()))?;

        let (plugin_name, surfaces) = match hello {
            PluginMessage::Hello {
                v,
                kind,
                name,
                surfaces,
            } => {
                if v != PROTOCOL_VERSION {
                    let _ = child.kill();
                    return Err(Refusal::Version {
                        theirs: v,
                        ours: PROTOCOL_VERSION,
                    });
                }
                if kind != "output" {
                    let _ = child.kill();
                    return Err(Refusal::NoGreeting);
                }
                (name, surfaces)
            }
            _ => {
                let _ = child.kill();
                return Err(Refusal::NoGreeting);
            }
        };

        if !surfaces.iter().any(|s| s == surface_kind) {
            let _ = child.kill();
            return Err(Refusal::UnsupportedSurface {
                wanted: surface_kind.to_string(),
                offered: surfaces,
            });
        }

        // 2. Send Open message
        let open_msg = HostMessage::Open {
            surface: surface_kind.to_string(),
            width,
            height,
            format: format.to_string(),
        };
        let open_line =
            serde_json::to_string(&open_msg).map_err(|e| Refusal::Process(e.to_string()))?;
        writeln!(stdin, "{open_line}")
            .map_err(|e| Refusal::Process(format!("stdin write: {e}")))?;
        stdin
            .flush()
            .map_err(|e| Refusal::Process(format!("stdin flush: {e}")))?;

        // 3. Await Ready confirmation
        let ready = msg_rx
            .recv_timeout(GREETING_TIMEOUT)
            .map_err(|_| Refusal::Timeout("waiting for plugin Ready confirmation".into()))?;

        let server_name = match ready {
            PluginMessage::Ready { server_name } => server_name,
            other => {
                let _ = child.kill();
                return Err(Refusal::NotReady(format!("{other:?}")));
            }
        };

        // 4. Setup background non-blocking writer thread
        let (tx, rx) = sync_channel::<HostMessage>(FRAME_QUEUE_CAPACITY);
        let host_dropped = Arc::new(AtomicU64::new(0));
        let connected_clients = Arc::new(AtomicU64::new(0));
        let plugin_dropped = Arc::new(AtomicU64::new(0));
        let alive = Arc::new(AtomicBool::new(true));

        let writer_alive = Arc::clone(&alive);
        std::thread::spawn(move || {
            let mut stdin = stdin;
            while let Ok(msg) = rx.recv() {
                let line = match serde_json::to_string(&msg) {
                    Ok(l) => l,
                    Err(_) => continue,
                };
                if writeln!(stdin, "{line}").is_err() || stdin.flush().is_err() {
                    writer_alive.store(false, Ordering::Release);
                    break;
                }
                if matches!(msg, HostMessage::Close) {
                    break;
                }
            }
        });

        // 5. Reader thread for status tracking
        let clients_ref = Arc::clone(&connected_clients);
        let dropped_ref = Arc::clone(&plugin_dropped);
        let reader_alive = Arc::clone(&alive);
        std::thread::spawn(move || {
            while let Ok(msg) = msg_rx.recv() {
                if let PluginMessage::Status { clients, dropped } = msg {
                    clients_ref.store(clients as u64, Ordering::Release);
                    dropped_ref.store(dropped, Ordering::Release);
                }
            }
            reader_alive.store(false, Ordering::Release);
        });

        Ok(Self {
            child,
            name: plugin_name,
            server_name,
            tx,
            host_dropped,
            connected_clients,
            plugin_dropped,
            alive,
        })
    }

    /// Dispatches a rendered frame to the plugin.
    ///
    /// Never blocks the caller (*the window never waits*). If the child process is busy
    /// or the buffer is full, the frame is dropped immediately and counted.
    pub fn send_frame(&self, index: u64, surface_id: u64, width: u32, height: u32) -> bool {
        if !self.alive.load(Ordering::Acquire) {
            return false;
        }
        let msg = HostMessage::Frame {
            index,
            surface_id,
            width,
            height,
        };
        match self.tx.try_send(msg) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                self.host_dropped.fetch_add(1, Ordering::Relaxed);
                false
            }
            Err(TrySendError::Disconnected(_)) => {
                self.alive.store(false, Ordering::Release);
                false
            }
        }
    }

    /// Informs the plugin of target dimension resize.
    pub fn resize(&self, width: u32, height: u32) {
        if self.alive.load(Ordering::Acquire) {
            let _ = self.tx.try_send(HostMessage::Resize { width, height });
        }
    }

    /// Returns human-readable plugin name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns published server name.
    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    /// Returns whether the plugin process is still alive and accepting commands.
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }

    /// Queries current streaming telemetry.
    pub fn telemetry(&self) -> PluginTelemetry {
        PluginTelemetry {
            clients: self.connected_clients.load(Ordering::Acquire) as u32,
            plugin_dropped: self.plugin_dropped.load(Ordering::Acquire),
            host_dropped: self.host_dropped.load(Ordering::Acquire),
        }
    }

    #[cfg(target_os = "windows")]
    /// Returns the raw process handle of the child plugin process.
    pub fn child_raw_handle(&self) -> std::os::windows::io::RawHandle {
        use std::os::windows::io::AsRawHandle;
        self.child.as_raw_handle()
    }
}

impl Drop for OutputPlugin {
    fn drop(&mut self) {
        let _ = self.tx.try_send(HostMessage::Close);
        // Allow brief moment for clean exit before forcing kill
        let start = Instant::now();
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if start.elapsed() > Duration::from_millis(50) => {
                    let _ = self.child.kill();
                    break;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(5)),
                Err(_) => break,
            }
        }
    }
}

#[cfg(test)]
mod tests;
