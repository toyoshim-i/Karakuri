//! Dynamic discovery and probing of out-of-process output plugins.
//!
//! Scans a plugins directory (resolved via [`crate::places::plugins`]), launches candidates
//! to read their `Hello` greeting, and captures self-reported plugin name and supported
//! surface types.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{HostMessage, PluginMessage, Refusal, PROTOCOL_VERSION};
use crate::places::Plugins;

const PROBE_TIMEOUT: Duration = Duration::from_millis(1500);

/// An out-of-process output plugin discovered on the system and self-reported via `Hello`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPlugin {
    /// Absolute or executable path to the plugin binary.
    pub path: PathBuf,
    /// Human-readable name self-reported by the plugin (e.g. "spout", "syphon").
    pub name: String,
    /// Surface mechanisms supported by the plugin (e.g. "dxgi", "iosurface").
    pub surfaces: Vec<String>,
}

impl DiscoveredPlugin {
    /// Returns whether this plugin supports a given surface mechanism (e.g. "dxgi" or "iosurface").
    pub fn supports_surface(&self, surface: &str) -> bool {
        self.surfaces.iter().any(|s| s == surface)
    }

    /// Formatted display name (capitalized).
    pub fn display_name(&self) -> String {
        let mut chars = self.name.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => "Plugin".to_string(),
        }
    }
}

/// Probes an executable file by launching it and waiting for the `Hello` greeting.
///
/// Returns `Ok(DiscoveredPlugin)` if the process speaks the Karakuri output plugin wire protocol,
/// or `Err(Refusal)` if it fails, times out, or is not a compatible plugin.
pub fn probe_plugin(path: &Path) -> Result<DiscoveredPlugin, Refusal> {
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| Refusal::Process(format!("{}: {e}", path.display())))?;

    let stdin = child.stdin.take();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Refusal::Process("failed to capture child stdout".into()))?;

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        if let Ok(n) = reader.read_line(&mut line) {
            if n > 0 {
                let trimmed = line.trim();
                if let Ok(msg) = serde_json::from_str::<PluginMessage>(trimmed) {
                    let _ = tx.send(msg);
                }
            }
        }
    });

    let msg = match rx.recv_timeout(PROBE_TIMEOUT) {
        Ok(m) => m,
        Err(_) => {
            let _ = child.kill();
            return Err(Refusal::Timeout(format!(
                "probing plugin `{}` timed out",
                path.display()
            )));
        }
    };

    // Politely close child stdin if possible, then ensure it terminates
    if let Some(mut stdin) = stdin {
        let close_msg = HostMessage::Close;
        if let Ok(line) = serde_json::to_string(&close_msg) {
            let _ = writeln!(stdin, "{line}");
            let _ = stdin.flush();
        }
    }

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > Duration::from_millis(50) => {
                let _ = child.kill();
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(_) => break,
        }
    }

    match msg {
        PluginMessage::Hello {
            v,
            kind,
            name,
            surfaces,
        } => {
            if v != PROTOCOL_VERSION {
                return Err(Refusal::Version {
                    theirs: v,
                    ours: PROTOCOL_VERSION,
                });
            }
            if kind != "output" {
                return Err(Refusal::NoGreeting);
            }
            Ok(DiscoveredPlugin {
                path: path.to_path_buf(),
                name,
                surfaces,
            })
        }
        _ => Err(Refusal::NoGreeting),
    }
}

/// Discovers all compatible output plugins in the given `plugins` directory.
pub fn discover_plugins(plugins: &Plugins) -> Vec<DiscoveredPlugin> {
    let Ok(executables) = plugins.list_executables() else {
        return Vec::new();
    };
    let mut discovered = Vec::new();
    for exe in executables {
        if let Ok(plugin) = probe_plugin(&exe) {
            discovered.push(plugin);
        }
    }
    discovered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_capitalizes_first_letter() {
        let p = DiscoveredPlugin {
            path: PathBuf::from("spout.exe"),
            name: "spout".to_string(),
            surfaces: vec!["dxgi".to_string()],
        };
        assert_eq!(p.display_name(), "Spout");
        assert!(p.supports_surface("dxgi"));
        assert!(!p.supports_surface("iosurface"));
    }
}
