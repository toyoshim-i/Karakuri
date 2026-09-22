use crate::hash::Hash;
use std::time::SystemTime;

/// A Set the store holds, as [`Store::list_sets`] found it: what to hand
/// [`Store::read_set`], and when that file was last written.
///
/// The time is a field rather than something a caller is left to read out of
/// the id. Ids auto-named after a timestamp do sort by time, which is exactly
/// what makes the assumption tempting and wrong — the moment an operator names
/// one `drift_01` it sorts beside the others and nowhere near when it was made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetEntry {
    pub id: String,
    pub written: SystemTime,
}

/// Metadata for an arrangement stored on disk, including its name and last-modified timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrangementEntry {
    pub name: String,
    pub written: SystemTime,
}

/// A procedure the operator has kept, as [`Store::list_procedures`] found it:
/// what to hand [`Store::read_procedure`], and when that file was last written.
///
/// `name` rather than `id`, which is [`ArrangementEntry`]'s own word and
/// carries its argument: a Set is ordinarily filed under a stamp nobody chose,
/// and a procedure never is — it is here because somebody pressed `keep` on a
/// node and said what to call it.
///
/// No `kind` field, and it is left off rather than dropped: what layer the file
/// declares is a line of the language, and [`Store::list_procedures`] is where
/// the reason this module does not read one is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcedureEntry {
    pub name: String,
    pub written: SystemTime,
}

/// An artifact the store holds, as [`Store::list_artifacts`] found it.
///
/// `has_meta` false is an ordinary artifact and not a damaged one: metadata is
/// derived, so anything put by a build older than the card format has none
/// until something regenerates it — the same state [`Store::read_meta`] returns
/// `NotFound` for. Carrying it as a flag is what lets a caller ask the question
/// in bulk instead of calling `read_meta` per artifact and reading the answer
/// out of an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactEntry {
    pub hash: Hash,
    pub has_meta: bool,
}
