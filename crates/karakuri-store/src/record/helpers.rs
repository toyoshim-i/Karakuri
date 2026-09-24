#[allow(unused_imports)]
use super::types::*;
use super::variants::*;

/// Category of file vocabulary to which a record belongs.
enum Vocabulary {
    /// Persistent Set state (`.kbset`).
    Set,
    /// Per-frame transient event (tick, audio analysis, tempo).
    Frame,
    /// Session and mixer state (gain, master controls, transport, selection).
    Session,
    /// Artifact metadata declarations (`<hash>.meta.ndjson`).
    Metadata,
    /// Unresolved authoring form of a Set (`.kset`).
    Authoring,
    /// Unrecognized record type preserved for round-trip compatibility.
    Unknown,
}

impl Record {
    /// Returns true if this record belongs in a persistent Set file (`.kbset`).
    ///
    /// Unknown records return true to preserve unrecognized fields across round-trips.
    pub fn is_set_state(&self) -> bool {
        matches!(self.vocabulary(), Vocabulary::Set | Vocabulary::Unknown)
    }

    /// Returns true if this record represents per-frame transient measurement or timing.
    pub fn is_measurement(&self) -> bool {
        matches!(self.vocabulary(), Vocabulary::Frame)
    }

    /// Returns the vocabulary classification for this record.
    fn vocabulary(&self) -> Vocabulary {
        match self {
            Record::Header { .. }
            | Record::Set { .. }
            | Record::Slot { .. }
            | Record::Capacity { .. }
            | Record::Param { .. }
            | Record::Bind { .. }
            | Record::Camera { .. }
            | Record::Merge { .. }
            | Record::Seed { .. }
            | Record::Edge { .. }
            | Record::Src { .. } => Vocabulary::Set,
            Record::Part { .. } => Vocabulary::Authoring,
            Record::Tick { .. } | Record::Audio { .. } | Record::Tempo { .. } => Vocabulary::Frame,
            Record::Gain { .. }
            | Record::Opacity { .. }
            | Record::Mute { .. }
            | Record::Solo { .. }
            | Record::Online { .. }
            | Record::Blend { .. }
            | Record::Residency { .. }
            | Record::Policy { .. }
            | Record::Look { .. }
            | Record::MasterOut { .. }
            | Record::MasterChain(_)
            | Record::Canvas { .. }
            | Record::Procedure { .. }
            | Record::Authority { .. }
            | Record::Ride { .. }
            | Record::Source { .. }
            | Record::Transport { .. }
            | Record::Transition { .. }
            | Record::Select { .. }
            | Record::Mask { .. }
            | Record::Save { .. } => Vocabulary::Session,
            Record::Meta { .. }
            | Record::ParamDecl { .. }
            | Record::CapacityDecl { .. }
            | Record::Emit { .. } => Vocabulary::Metadata,
            Record::Unknown => Vocabulary::Unknown,
        }
    }

    /// Returns true if this record belongs in an artifact's metadata card (`<hash>.meta.ndjson`).
    pub fn is_metadata(&self) -> bool {
        matches!(self.vocabulary(), Vocabulary::Metadata)
    }

    /// Returns true if this record belongs to the authoring form (`.kset`) rather than resolved Set state.
    pub fn is_authoring(&self) -> bool {
        matches!(self.vocabulary(), Vocabulary::Authoring)
    }
}
