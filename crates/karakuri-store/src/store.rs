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
//!
//! `<hash>` is the artifact's content address rendered as bare lowercase
//! hex (no `sha256:` prefix and no colon) so it is safe as a path component
//! on every platform the store might run on.

use std::fs;
use std::path::PathBuf;

use crate::hash::Hash;
use crate::ndjson::{self, Line};
use crate::project;

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
    /// A Set file carries no time — see `docs/ir-spec.md`, Set file format.
    /// `tick` was the only such record when this was named; `audio` and `tempo`
    /// are the same kind of thing, so the check is `Record::is_set_state` and the
    /// message says so rather than naming one of the three.
    #[error(
        "set files cannot contain a tick, audio or tempo record — a Set file carries no time \
         (offending record at index {index})"
    )]
    TickInSet { index: usize },
}

/// A content-addressed store of `.kir` artifacts, plus the Set files and
/// session streams that reference them.
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// Establish the store layout under `root`, creating any directories
    /// that do not exist yet. Safe to call repeatedly on the same root.
    pub fn open(root: impl Into<PathBuf>) -> Result<Store, StoreError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        fs::create_dir_all(root.join("previews"))?;
        fs::create_dir_all(root.join("sets"))?;
        fs::create_dir_all(root.join("sessions"))?;
        Ok(Store { root })
    }

    fn artifact_path(&self, hash: &Hash) -> PathBuf {
        self.root.join(format!("{}.kir", hash.short(64)))
    }

    fn set_path(&self, id: &str) -> PathBuf {
        self.root.join("sets").join(format!("{id}.set.ndjson"))
    }

    fn session_path(&self, stamp: &str) -> PathBuf {
        self.root.join("sessions").join(format!("{stamp}.ndjson"))
    }

    /// Store `.kir` source, content-addressed by its SHA-256. Writing the
    /// same source twice is a no-op the second time: the artifact already
    /// on disk is never rewritten, so it can never be corrupted by a
    /// concurrent or repeated `put`.
    pub fn put_artifact(&self, source: &[u8]) -> Result<Hash, StoreError> {
        let hash = Hash::of(source);
        let path = self.artifact_path(&hash);
        if !path.exists() {
            ndjson::write_atomic(&path, source)?;
        }
        Ok(hash)
    }

    /// Fetch `.kir` source by content address. `StoreError::NotFound` if no
    /// artifact has been put under that hash.
    pub fn get_artifact(&self, hash: &Hash) -> Result<Vec<u8>, StoreError> {
        let path = self.artifact_path(hash);
        fs::read(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => StoreError::NotFound(*hash),
            _ => StoreError::Io(e),
        })
    }

    /// Read a Set file (`sets/<id>.set.ndjson`).
    pub fn read_set(&self, id: &str) -> Result<Vec<Line>, StoreError> {
        ndjson::read(&self.set_path(id))
    }

    /// Write a Set file. Rejects any line carrying a `Tick` record — a Set
    /// file is a state projection and carries no time — rather than
    /// trusting the caller to have stripped ticks already.
    pub fn write_set(&self, id: &str, lines: &[Line]) -> Result<(), StoreError> {
        if let Some(index) = lines.iter().position(|l| !l.record().is_set_state()) {
            return Err(StoreError::TickInSet { index });
        }
        ndjson::write(&self.set_path(id), lines)
    }

    /// Read a session stream (`sessions/<stamp>.ndjson`).
    pub fn read_session(&self, stamp: &str) -> Result<Vec<Line>, StoreError> {
        ndjson::read(&self.session_path(stamp))
    }

    /// Write a session stream. Unlike [`Store::write_set`], ticks are
    /// expected here — a session is the timeline, ticks and all.
    pub fn write_session(&self, stamp: &str, lines: &[Line]) -> Result<(), StoreError> {
        ndjson::write(&self.session_path(stamp), lines)
    }

    /// Save a live session as a Set: the session stream with ticks dropped
    /// and the state folded down, last write wins per layer and key. See
    /// [`project::project`] and `docs/ir-spec.md`, Session stream format.
    pub fn save_session_as_set(&self, set_id: &str, session: &[Line]) -> Result<(), StoreError> {
        self.write_set(set_id, &project::project(session))
    }
}
