use super::id::ControlId;

/// Unified descriptor holding metadata for an interactive control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlDescriptor {
    /// Strongly-typed control ID.
    pub id: ControlId,
    /// The name of the probe in `PROBES` that hit-tests this control.
    pub probe_name: &'static str,
    /// Human-readable label / description.
    pub label: &'static str,
    /// Optional default keyboard shortcut (e.g. "g", "z", "r", "k", "b", "tab", "esc").
    pub hotkey: Option<&'static str>,
    /// Bound operation name if applicable.
    pub operation_title: Option<&'static str>,
    /// Primary gesture or interaction method (e.g. "Click", "Drag", "Click or Drag", "Key").
    pub action: Option<&'static str>,
    /// Associated MCP tool name or mutation policy if agent-accessible.
    pub mcp_policy: Option<&'static str>,
}
