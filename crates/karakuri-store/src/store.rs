//! The on-disk artifact store.
//!
//! Layout under the store root (`library/` by default):
//!
//! ```text
//! <hash>.kir              source, immutable
//! <hash>.meta.ndjson      regenerated metadata
//! previews/<hash>.mp4
//! sets/<id>.set.ndjson
//! sessions/<stamp>.ndjson
//! ```

use std::path::PathBuf;

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
}

/// A content-addressed store of `.kir` artifacts.
pub struct Store {
    #[allow(dead_code)]
    root: PathBuf,
}

impl Store {
    pub fn open(_root: impl Into<PathBuf>) -> Result<Store, StoreError> {
        todo!("R1: store")
    }
}
