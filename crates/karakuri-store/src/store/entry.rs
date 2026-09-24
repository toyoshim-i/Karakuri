use crate::hash::Hash;
use std::time::SystemTime;

/// A stored Set entry returned by [`Store::list_sets`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetEntry {
    /// Unique identifier of the Set.
    pub id: String,
    /// Last-modified timestamp of the `.kbset` file on disk.
    pub written: SystemTime,
}

/// Metadata for an arrangement stored on disk, including its name and last-modified timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrangementEntry {
    pub name: String,
    pub written: SystemTime,
}

/// A stored procedure entry returned by [`Store::list_procedures`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcedureEntry {
    /// Name of the kept procedure file (excluding extension).
    pub name: String,
    /// Last-modified timestamp of the `.kir` file on disk.
    pub written: SystemTime,
}

/// An artifact entry returned by [`Store::list_artifacts`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactEntry {
    /// Content hash address of the artifact.
    pub hash: Hash,
    /// Whether corresponding `.meta.ndjson` metadata exists on disk.
    pub has_meta: bool,
}
