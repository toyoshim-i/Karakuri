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

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

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

    /// **List the Sets the store holds**, in ascending id order, each with the
    /// time its file was last written.
    ///
    /// **The time comes from the filesystem because the file has none.** A Set
    /// file is a state projection and carries no time at all — that is what
    /// [`StoreError::TickInSet`] exists to enforce — so there is nothing inside
    /// one to sort by, and the mtime is not a second-best here but the only
    /// record that exists of when a Set was saved. [`Store::write_set`] renames
    /// a complete file into place, so what that mtime marks is the moment the
    /// Set became readable rather than the moment some writer opened a file.
    ///
    /// **Ordered by id and not by recency**, though "the one I saved last" is
    /// the question this method is mostly asked. Two Sets written within one
    /// tick of a coarse filesystem clock carry the same mtime, and a sort whose
    /// keys tie falls back to whatever `read_dir` handed us — which is not an
    /// order, and would differ between two calls on an unchanged store. Ids are
    /// unique by construction, so ordering on them is total and repeatable;
    /// recency is a `sort_by_key(|e| e.written)` away, and the field to do it
    /// with is on every entry.
    ///
    /// **A name the layout does not claim is skipped, not repaired.** The
    /// directory holds `<id>.set.ndjson`; an editor's backup, a `.tmp` left by
    /// a write that died, a subdirectory someone made — those belong to whoever
    /// put them there, and reporting one as a Set under a truncated id would
    /// invent a library entry [`Store::read_set`] cannot open.
    ///
    /// An empty store lists nothing, and that is not an error. A `sets/`
    /// directory removed under the store *is* one: the caller asked what is
    /// there and we have no answer, and an empty `Vec` would say "nothing is
    /// kept" to a question we could not read.
    pub fn list_sets(&self) -> Result<Vec<SetEntry>, StoreError> {
        let mut out = Vec::new();
        for entry in fs::read_dir(self.root.join("sets"))? {
            let entry = entry?;
            let name = entry.file_name();
            // Non-UTF-8 fails `to_str` and falls out of the listing with
            // everything else the layout does not claim — no lossy repair, and
            // no unwrap for a hostile name to trip.
            let Some(id) = name.to_str().and_then(|n| n.strip_suffix(".set.ndjson")) else {
                continue;
            };
            if entry.file_type()?.is_dir() {
                continue;
            }
            out.push(SetEntry {
                id: id.to_string(),
                written: entry.metadata()?.modified()?,
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// **List the artifacts the store holds, and say which of them have a
    /// metadata card**, in ascending hash order.
    ///
    /// **One listing and not two.** The layout puts `<hash>.kir` and
    /// `<hash>.meta.ndjson` in the same directory, so the pass that finds the
    /// artifacts has already seen the cards; a separate `list_carded` would
    /// read the directory a second time to recover what this one would have
    /// thrown away. Carded and uncarded are then the same list read two ways —
    /// filter one way to ask which artifacts a search index can already
    /// describe, the other to ask what a regeneration pass has left to do.
    ///
    /// **A card is not an artifact.** A `<hash>.meta.ndjson` with no `.kir`
    /// beside it is exactly the stray file [`Store::write_meta`] declines to
    /// prevent, and it does not appear here: the `.kir` files are the
    /// population and the cards only decorate them. A directory is not an
    /// artifact either, whatever it happens to be named.
    ///
    /// **And a name has to be one this store would have written**, not merely
    /// one that parses. `<UPPERCASE HEX>.kir` decodes to a perfectly good
    /// address — whose `.kir` path is then the *lowercase* name, so listing it
    /// would hand back a hash [`Store::get_artifact`] cannot find. Rendering the
    /// parsed address out again and requiring it to equal the stem costs one
    /// string compare and makes every hash listed a hash the rest of this API
    /// works on.
    ///
    /// Ordering is by the address itself, which for hex is the order a reader
    /// scanning the column expects. As with [`Store::list_sets`], an empty
    /// store lists nothing and a missing root is an error.
    pub fn list_artifacts(&self) -> Result<Vec<ArtifactEntry>, StoreError> {
        let mut sources = BTreeSet::new();
        let mut cards = BTreeSet::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let stem = if let Some(stem) = name.strip_suffix(".kir") {
                Some((stem, &mut sources))
            } else {
                name.strip_suffix(".meta.ndjson").map(|s| (s, &mut cards))
            };
            let Some((stem, set)) = stem else { continue };
            let Some(hash) = parse_hash_stem(stem) else {
                continue;
            };
            if entry.file_type()?.is_dir() {
                continue;
            }
            set.insert(hash);
        }
        // `BTreeSet` has already put the addresses in order, and having each
        // one at most once is the same property that makes the card lookup a
        // membership test rather than a scan.
        Ok(sources
            .into_iter()
            .map(|hash| ArtifactEntry {
                has_meta: cards.contains(&hash),
                hash,
            })
            .collect())
    }
}

/// Read a `<hash>` path component back as the address it names, or `None` if
/// the name is not one this store would have written.
///
/// The round trip through [`Hash::short`] is the whole check: parsing alone
/// accepts spellings — uppercase hex, most obviously — that never name a file
/// the store put there, and a listing that reported one would be reporting an
/// artifact nothing can read back.
fn parse_hash_stem(stem: &str) -> Option<Hash> {
    let hash: Hash = format!("sha256:{stem}").parse().ok()?;
    (hash.short(64) == stem).then_some(hash)
}

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
