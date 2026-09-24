use super::Store;
use crate::hash::Hash;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("malformed record on line {line}: {source}")]
    Record {
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error("no artifact for {0}")]
    NotFound(Hash),
    /// No arrangement file exists under the given name.
    #[error("no arrangement named `{0}`")]
    NoArrangement(String),
    /// No procedure file exists under the given name.
    #[error("no procedure named `{0}`")]
    NoProcedure(String),
    /// A procedure with the requested name already exists in the store.
    #[error(
        "a procedure named `{0}` is already kept — a keep never overwrites one, so name this \
         one something else"
    )]
    ProcedureTaken(String),
    /// The referenced Set identifier was not found in the store.
    #[error("no Set named `{0}` in this store, so there is nothing to star")]
    NoSet(String),
    /// The favourites file on disk could not be parsed as a list of Set identifiers.
    #[error("`{}` is not a list of Set ids: {source}", Store::FAVOURITES_FILE)]
    Favourites {
        #[source]
        source: serde_json::Error,
    },
    /// The slot policies file on disk is corrupted or unparseable.
    #[error("`{}` is not valid slot policies: {source}", Store::POLICIES_FILE)]
    Policies {
        #[source]
        source: serde_json::Error,
    },
    /// Set files cannot contain runtime temporal records (tick, audio, tempo).
    #[error(
        "set files cannot contain a tick, audio or tempo record — a Set file carries no time \
         (offending record at index {index})"
    )]
    TickInSet { index: usize },
    /// Set files cannot contain metadata declarations belonging to `<hash>.meta.ndjson`.
    #[error(
        "set files cannot contain a meta, param_decl, capacity_decl or emit record — those \
         describe what an artifact declares and belong in its `<hash>.meta.ndjson` \
         (offending record at index {index})"
    )]
    MetaInSet { index: usize },
    /// Set files under `sets/` must be resolved `.kbset` files and cannot contain relative `part` records.
    #[error(
        "set files cannot contain a part record — a `part` names its `.kir` by relative path \
         and belongs to the authoring form (`.kset`), where `sets/` holds the resolved form \
         (`.kbset`) and only that; resolve it first (offending record at index {index})"
    )]
    PartInSet { index: usize },
}
