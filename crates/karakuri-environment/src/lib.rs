//! External environment integration layer for Karakuri.
//! Provides audio input, MIDI control, file watching, compilation, and session services.

// Module declarations for platform integration services.
pub mod audio;
pub mod clock;
pub mod compile;
pub mod history;
pub mod meta;
pub mod midi;
pub mod mix;
pub mod output_plugin;
pub mod places;
pub mod render;
pub mod scratch;
pub mod session;
pub mod setfile;
pub mod tempo_source;
pub mod watch;

/// Shared operator gate state for automated operation routes (ADR-0235, ADR-0236).
#[derive(Debug, Clone, Default)]
pub struct Opening(std::sync::Arc<std::sync::RwLock<karakuri_operation::gate::Open>>);

impl Opening {
    /// Returns a new Opening handle with all operation classes closed.
    pub fn closed() -> Opening {
        Opening::default()
    }

    /// Reads current gate permissions, defaulting to closed if poisoned.
    pub fn read(&self) -> karakuri_operation::gate::Open {
        self.0
            .read()
            .map(|open| *open)
            .unwrap_or(karakuri_operation::gate::Open::CLOSED)
    }

    /// Updates gate permissions from an operator input surface.
    pub fn set(&self, open: karakuri_operation::gate::Open) {
        if let Ok(mut held) = self.0.write() {
            *held = open;
        }
    }
}

/// Slot-level MCP modification policies and mix activity.
#[derive(Debug, Clone)]
pub struct SlotPolicies(std::sync::Arc<std::sync::RwLock<[karakuri_operation::SlotAccess; 4]>>);

impl Default for SlotPolicies {
    fn default() -> Self {
        Self(std::sync::Arc::new(std::sync::RwLock::new(
            [karakuri_operation::SlotAccess::default(); 4],
        )))
    }
}

impl SlotPolicies {
    /// Initialized with default access (policy = Auto, in_mix = false).
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads the access status for a slot.
    pub fn access(&self, slot: usize) -> karakuri_operation::SlotAccess {
        self.0
            .read()
            .ok()
            .and_then(|guard| guard.get(slot).copied())
            .unwrap_or_default()
    }

    /// Reads the policy configured for a slot.
    pub fn policy(&self, slot: usize) -> karakuri_operation::SlotPolicy {
        self.access(slot).policy
    }

    /// Checks if a slot is writable by MCP, returning structured refusal details if rejected.
    pub fn check_writable_detail(
        &self,
        slot: usize,
    ) -> Result<(), karakuri_operation::RefusalDetail> {
        let access = self.access(slot);
        if access.is_writable() {
            Ok(())
        } else {
            Err(access
                .refusal_detail(slot)
                .unwrap_or_else(|| karakuri_operation::RefusalDetail {
                    code: karakuri_operation::RefusalCode::SlotPolicyOff,
                    message: format!("slot {slot} is not writable by MCP"),
                    slot: Some(slot),
                    deck: u8::try_from(slot).ok(),
                    lane: None,
                    class: None,
                    policy: Some(access.policy),
                    in_mix: Some(access.in_mix),
                }))
        }
    }

    /// Checks if a slot is writable by MCP, returning an error message if refused.
    pub fn check_writable(&self, slot: usize) -> Result<(), String> {
        self.check_writable_detail(slot).map_err(|d| d.message)
    }

    /// Sets the policy for a slot.
    pub fn set_policy(&self, slot: usize, policy: karakuri_operation::SlotPolicy) {
        if let Ok(mut guard) = self.0.write() {
            if let Some(entry) = guard.get_mut(slot) {
                entry.policy = policy;
            }
        }
    }

    /// Sets whether a slot is contributing to the mix.
    pub fn set_in_mix(&self, slot: usize, in_mix: bool) {
        if let Ok(mut guard) = self.0.write() {
            if let Some(entry) = guard.get_mut(slot) {
                entry.in_mix = in_mix;
            }
        }
    }

    /// Reads all 4 slot access states.
    pub fn all(&self) -> [karakuri_operation::SlotAccess; 4] {
        self.0.read().map(|guard| *guard).unwrap_or_default()
    }
}

/// Maximum duration to await in-flight saves during shutdown before terminating.
pub const SAVE_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// Identifies the originator of a save operation to determine library vs sandbox target directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// The operator's own act: `--save-set`, the `k` key, a control they pressed.
    Operator,
    /// A model, over MCP.
    Model,
}

/// Formats standard refusal error message when referencing an invalid slot index.
pub fn no_such_slot(slot: usize, slot_count: usize) -> String {
    match slot_count {
        0 => format!("no slot {slot}: this deck holds none"),
        n => format!("no slot {slot}: this deck holds slots 0-{}", n - 1),
    }
}

/// Formats standard refusal error message when referencing an invalid renderer index within a slot.
pub fn no_such_renderer(slot: usize, at: usize, count: usize) -> String {
    match count {
        0 => format!("no renderer {at}: slot {slot} draws with none"),
        n => format!(
            "no renderer {at}: slot {slot} draws with renderers 0-{}",
            n - 1
        ),
    }
}

/// Error message for parameter names not declared by the Set in `slot` (ADR-0268, Principle 0083).
pub fn no_such_param(slot: usize, key: &str) -> String {
    format!("no parameter `{key}`: the Set in slot {slot} declares none by that name")
}

/// Formats explanatory error message when a slot has no persistable sources in the store.
pub fn nothing_to_save(slot: usize, loaded_set: Option<&str>, no_files: bool) -> String {
    match loaded_set {
        Some(id) if no_files => format!(
            "slot {slot}: nothing to save — it was filled from set `{id}` by hash, with no \
             files behind it and nothing able to rebuild it. Start the run with `--watch` \
             or `--mcp` and this slot saves like any other"
        ),
        _ => format!(
            "slot {slot}: nothing to save — this slot's sources are not in the store, which \
             was said at startup, and no rebuild of it has landed since"
        ),
    }
}

/// Callback receiver notified when a save request has been accepted.
pub trait SaveReply {
    /// Delivers notification that save operation with message `said` was accepted.
    fn accepted(&self, said: &str);
}

/// Notifies operator/client that a save operation was accepted and returns target save identifier.
pub fn accepted_save(
    slot: usize,
    asked: Asked,
    id: Option<String>,
    sources: &setfile::Sources,
    root: &std::path::Path,
    reply: Option<&dyn SaveReply>,
) -> String {
    let id = filed_as(asked, id);
    let said = format!(
        "slot {slot}: saving {} node{} as set `{id}` in {}",
        sources.len(),
        if sources.len() == 1 { "" } else { "s" },
        match asked {
            Asked::Operator => root.display().to_string(),
            // Returns concrete subdirectory path for model-initiated saves.
            Asked::Model => root
                .join(karakuri_store::store::Store::SANDBOX)
                .display()
                .to_string(),
        }
    );
    eprintln!("{said}");
    if let Some(reply) = reply {
        reply.accepted(&said);
    }
    id
}

/// Resolves the save file identifier based on caller type and optional user-supplied name (ADR-0128).
///
/// Operator saves preserve explicit IDs or generate timestamps. Model saves always prefix a timestamp.
fn filed_as(asked: Asked, id: Option<String>) -> String {
    match (asked, id) {
        (Asked::Operator, Some(id)) => id,
        (Asked::Operator, None) => history::stamped_id(),
        (Asked::Model, None) => history::stamped_id(),
        (Asked::Model, Some(name)) => format!("{}_{name}", history::stamped_id()),
    }
}
