//! The on-disk artifact store.
//!
//! Layout under the store root (`library/` by default):
//!
//! ```text
//! <hash>.kir              source, immutable
//! <hash>.meta.ndjson      regenerated metadata
//! thumbnails/<hash>.mp4
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
    /// A Set file says what a Set's values *are*; a `meta`, `param_decl`,
    /// `capacity_decl` or `emit` record says what an artifact *declares*, and
    /// belongs in `<hash>.meta.ndjson`. Refused on the same terms as a session
    /// record and with a different sentence, because the reason differs: the
    /// check is `Record::is_metadata` rather than `!Record::is_set_state`, and
    /// running the two together under one message would tell an operator a
    /// declaration was rejected for carrying time.
    #[error(
        "set files cannot contain a meta, param_decl, capacity_decl or emit record — those \
         describe what an artifact declares and belong in its `<hash>.meta.ndjson` \
         (offending record at index {index})"
    )]
    MetaInSet { index: usize },
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
        // **`thumbnails/` and not `previews/`.** What lands here is the
        // library's stored asset, which the metadata vocabulary calls a
        // `thumbnail` — the deck's `preview` is a live audition of a running
        // slot and has nothing on disk. Nothing writes into this directory
        // yet; the name moved with the record's, so the path a `thumbnail`
        // record carries names a directory that exists.
        fs::create_dir_all(root.join("thumbnails"))?;
        fs::create_dir_all(root.join("sets"))?;
        fs::create_dir_all(root.join("sessions"))?;
        Ok(Store { root })
    }

    fn artifact_path(&self, hash: &Hash) -> PathBuf {
        self.root.join(format!("{}.kir", hash.short(64)))
    }

    /// Where an artifact's regenerated metadata lives: beside the `.kir` and
    /// under the same bare hex, so the two are one `ls` apart and a card can
    /// never be filed under a name its artifact does not have.
    fn meta_path(&self, hash: &Hash) -> PathBuf {
        self.root.join(format!("{}.meta.ndjson", hash.short(64)))
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

    /// **Write an artifact's metadata file** (`<hash>.meta.ndjson`).
    ///
    /// **Overwrites, where [`Store::put_artifact`] refuses to.** An artifact is
    /// immutable and its bytes are its address, so rewriting one can only ever
    /// corrupt it; metadata is *derived* — regenerated from the `.kir` plus a
    /// compile pass — so a later build that knows more writes a better card and
    /// the old one has no claim. The write is atomic all the same, so a reader
    /// meeting it mid-regeneration sees the whole of one version or the whole
    /// of the other.
    ///
    /// Nothing here checks that the artifact exists. The caller puts the source
    /// and then the card, and a card with no artifact is a stray file rather
    /// than a corruption — where a check would make every writer pay a read to
    /// prove something it just did.
    pub fn write_meta(&self, hash: &Hash, lines: &[Line]) -> Result<(), StoreError> {
        ndjson::write(&self.meta_path(hash), lines)
    }

    /// **Read an artifact's metadata file.** `StoreError::NotFound` where no
    /// card has been written for that hash — which is an ordinary state, not a
    /// damaged store: metadata is derived, and an artifact put by an older
    /// build has none until something regenerates it.
    pub fn read_meta(&self, hash: &Hash) -> Result<Vec<Line>, StoreError> {
        let path = self.meta_path(hash);
        if !path.exists() {
            return Err(StoreError::NotFound(*hash));
        }
        ndjson::read(&path)
    }

    /// Read a Set file (`sets/<id>.set.ndjson`).
    pub fn read_set(&self, id: &str) -> Result<Vec<Line>, StoreError> {
        ndjson::read(&self.set_path(id))
    }

    /// Write a Set file. Rejects any line carrying a `Tick` record — a Set
    /// file is a state projection and carries no time — rather than
    /// trusting the caller to have stripped ticks already.
    ///
    /// **And rejects an artifact's metadata on the same terms**, by the same
    /// scan and with a sentence of its own: a `param_decl` says what a
    /// procedure declares and a `param` says what this Set turned it to, and a
    /// Set file holding the first would be describing an artifact rather than a
    /// Set. Two questions asked rather than one predicate widened — see
    /// [`Record::is_set_state`](crate::record::Record::is_set_state), whose one
    /// answer already stands on three different reasons, and whose sentence
    /// names a tick. **Two questions and one classification**: both are read
    /// off the same exhaustive match, so a record cannot be metadata to one of
    /// them and Set state to the other.
    pub fn write_set(&self, id: &str, lines: &[Line]) -> Result<(), StoreError> {
        if let Some(index) = lines.iter().position(|l| l.record().is_metadata()) {
            return Err(StoreError::MetaInSet { index });
        }
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

    /// **Open a session stream for appending**, for a writer that produces the
    /// timeline as it happens rather than holding a whole set in memory.
    ///
    /// Not atomic, and deliberately not: [`Store::write_session`] renames a
    /// complete file into place, which is right for something written once and
    /// wrong for something written for an hour. A session appended to is
    /// readable up to its last complete line at every moment, and a run that
    /// dies mid-set leaves the set up to that point rather than nothing.
    pub fn append_session(&self, stamp: &str) -> Result<fs::File, StoreError> {
        Ok(fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.session_path(stamp))?)
    }

    /// Save a live session as a Set: the session stream with ticks dropped
    /// and the state folded down, last write wins per layer and key. See
    /// [`project::project`] and `docs/ir-spec.md`, Session stream format.
    pub fn save_session_as_set(&self, set_id: &str, session: &[Line]) -> Result<(), StoreError> {
        self.write_set(set_id, &project::project(session))
    }
}
