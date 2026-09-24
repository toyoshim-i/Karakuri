//! Wire messages exchanged between Karakuri host and output sink plugins.
//!
//! One line of ndjson per message across stdin (host -> plugin) and stdout (plugin -> host).
//! Stderr is reserved for diagnostics and human-readable logging.

use serde::{Deserialize, Serialize};

/// The wire protocol version this build speaks.
pub const PROTOCOL_VERSION: u32 = 1;

/// Messages sent from the host (Karakuri) to the output plugin via stdin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum HostMessage {
    /// Configure and open the output stream with a negotiated surface kind and dimensions.
    Open {
        /// Surface type negotiated with the plugin (e.g. "iosurface" on macOS, "dxgi" on Windows).
        surface: String,
        /// Frame width in pixels.
        width: u32,
        /// Frame height in pixels.
        height: u32,
        /// Pixel format descriptor (e.g. "bgra8unorm", "rgba8unorm").
        format: String,
    },
    /// Deliver a newly composited frame to the plugin.
    Frame {
        /// Monotonically increasing frame index.
        index: u64,
        /// Platform-native shareable surface identifier:
        /// - On macOS: `IOSurfaceID` (u32) resolvable via `IOSurfaceLookup`
        /// - On Windows: DXGI shared handle (`HANDLE`, 64-bit integer)
        surface_id: u64,
        /// Current frame width in pixels.
        width: u32,
        /// Current frame height in pixels.
        height: u32,
    },
    /// Inform the plugin that output target dimensions have changed.
    Resize {
        /// New frame width in pixels.
        width: u32,
        /// New frame height in pixels.
        height: u32,
    },
    /// Signal the plugin to gracefully flush buffers and terminate.
    Close,
    /// Unrecognised host message tag (forward compatibility).
    #[serde(other)]
    Unknown,
}

/// Messages sent from the output plugin to the host (Karakuri) via stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum PluginMessage {
    /// First message emitted by the plugin on startup.
    Hello {
        /// Protocol version spoken by the plugin.
        v: u32,
        /// Plugin classification; must be "output".
        kind: String,
        /// Human-readable plugin name (e.g. "syphon", "spout", "ndi").
        name: String,
        /// List of supported surface backings (e.g. ["iosurface"], ["dxgi"], ["shm"]).
        surfaces: Vec<String>,
    },
    /// Acknowledgment emitted after receiving `HostMessage::Open`.
    Ready {
        /// Server publication name visible to external consumers (e.g. "Karakuri").
        server_name: String,
    },
    /// Periodic or event-driven status report from the plugin.
    Status {
        /// Number of external receivers / clients currently attached to the output stream.
        clients: u32,
        /// Total number of dropped frames since stream start.
        dropped: u64,
    },
    /// Unrecognised plugin message tag (forward compatibility).
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_message_round_trip() {
        let msg = HostMessage::Frame {
            index: 42,
            surface_id: 108,
            width: 1920,
            height: 1080,
        };
        let serialized = serde_json::to_string(&msg).expect("serialize");
        assert_eq!(
            serialized,
            r#"{"t":"frame","index":42,"surface_id":108,"width":1920,"height":1080}"#
        );
        let parsed: HostMessage = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(parsed, msg);
    }

    #[test]
    fn plugin_message_round_trip() {
        let msg = PluginMessage::Hello {
            v: 1,
            kind: "output".to_string(),
            name: "syphon".to_string(),
            surfaces: vec!["iosurface".to_string()],
        };
        let serialized = serde_json::to_string(&msg).expect("serialize");
        assert_eq!(
            serialized,
            r#"{"t":"hello","v":1,"kind":"output","name":"syphon","surfaces":["iosurface"]}"#
        );
        let parsed: PluginMessage = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(parsed, msg);
    }

    #[test]
    fn unknown_tags_and_extra_fields_ignored() {
        let raw = r#"{"t":"future_message","future_field":123}"#;
        let host: HostMessage = serde_json::from_str(raw).expect("host deserialize");
        assert_eq!(host, HostMessage::Unknown);
        let plugin: PluginMessage = serde_json::from_str(raw).expect("plugin deserialize");
        assert_eq!(plugin, PluginMessage::Unknown);

        let extra_field = r#"{"t":"ready","server_name":"Karakuri","extra":true}"#;
        let ready: PluginMessage = serde_json::from_str(extra_field).expect("ready deserialize");
        assert_eq!(
            ready,
            PluginMessage::Ready {
                server_name: "Karakuri".to_string()
            }
        );
    }
}
