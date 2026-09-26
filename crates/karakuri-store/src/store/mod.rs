//! On-disk artifact store managing content-addressed `.kir` sources, sets, and session logs.
//! Content addresses are stored as 64-character lowercase hex without `sha256:` prefix.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use crate::hash::Hash;
use crate::ndjson::{self, Line};
use crate::project;

pub mod entry;
pub mod error;

pub use entry::*;
pub use error::*;

/// A content-addressed store of `.kir` artifacts, plus the Set files and
/// session streams that reference them.
pub struct Store {
    root: PathBuf,
}

/// Validates that a Set file does not contain metadata, authoring parts, or temporal records.
fn refuse_what_is_not_a_set(lines: &[Line]) -> Result<(), StoreError> {
    if let Some(index) = lines.iter().position(|l| l.record().is_metadata()) {
        return Err(StoreError::MetaInSet { index });
    }
    if let Some(index) = lines.iter().position(|l| l.record().is_authoring()) {
        return Err(StoreError::PartInSet { index });
    }
    if let Some(index) = lines.iter().position(|l| !l.record().is_set_state()) {
        return Err(StoreError::TickInSet { index });
    }
    Ok(())
}

impl Store {
    /// Establish the store layout under `root`, creating any directories that do
    /// not exist yet. Safe to call repeatedly on the same root.
    pub fn open(root: impl Into<PathBuf>) -> Result<Store, StoreError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        fs::create_dir_all(root.join("thumbnails"))?;
        fs::create_dir_all(root.join("sets"))?;
        fs::create_dir_all(root.join(Store::SANDBOX))?;
        fs::create_dir_all(root.join("sessions"))?;
        fs::create_dir_all(root.join("arrangements"))?;
        fs::create_dir_all(root.join(Store::PROCEDURES))?;
        Ok(Store { root })
    }

    fn artifact_path(&self, hash: &Hash) -> PathBuf {
        self.root.join(format!("{}.kir", hash.short(64)))
    }

    /// Where an artifact's regenerated metadata lives: beside the `.kir` and under
    /// the same bare hex, so the two are one `ls` apart and a card can never be
    /// filed under a name its artifact does not have.
    fn meta_path(&self, hash: &Hash) -> PathBuf {
        self.root.join(format!("{}.meta.ndjson", hash.short(64)))
    }

    /// File extension for resolved on-disk Set files (`.kbset`, ADR-0231).
    pub const SET_FILE_SUFFIX: &str = ".kbset";

    /// Subdirectory for model-generated sandbox Sets and procedures (ADR-0261, P-0096).
    pub const SANDBOX: &str = "sandbox";

    /// Subdirectory for operator-kept procedure source files (ADR-0338, P-0096).
    pub const PROCEDURES: &str = "procedures";

    /// File extension for kept `.kir` procedure source files.
    pub const PROCEDURE_FILE_SUFFIX: &str = ".kir";

    /// [`Store::arrangement_path`]'s sibling under [`Store::PROCEDURES`], built the
    /// same way and off the same suffix.
    fn procedure_path(&self, name: &str) -> PathBuf {
        self.root
            .join(Store::PROCEDURES)
            .join(format!("{name}{}", Store::PROCEDURE_FILE_SUFFIX))
    }

    fn set_path(&self, id: &str) -> PathBuf {
        self.root
            .join("sets")
            .join(format!("{id}{}", Store::SET_FILE_SUFFIX))
    }

    /// [`Store::set_path`]'s sibling under [`Store::SANDBOX`], built the same way
    /// and off the same suffix: what lands here is a Set file, and a second
    /// spelling of the extension is the drift [`Store::SET_FILE_SUFFIX`] exists to
    /// prevent.
    fn sandbox_path(&self, id: &str) -> PathBuf {
        self.root
            .join(Store::SANDBOX)
            .join(format!("{id}{}", Store::SET_FILE_SUFFIX))
    }

    fn session_path(&self, stamp: &str) -> PathBuf {
        self.root.join("sessions").join(format!("{stamp}.ndjson"))
    }

    fn arrangement_path(&self, name: &str) -> PathBuf {
        self.root
            .join("arrangements")
            .join(format!("{name}.arrangement.json"))
    }

    /// Store `.kir` source, content-addressed by its SHA-256. Writing the same
    /// source twice is a no-op the second time: the artifact already on disk is
    /// never rewritten, so it can never be corrupted by a concurrent or repeated
    /// `put`.
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

    /// Atomically writes an artifact's derived metadata file (`<hash>.meta.ndjson`).
    pub fn write_meta(&self, hash: &Hash, lines: &[Line]) -> Result<(), StoreError> {
        ndjson::write(&self.meta_path(hash), lines)
    }

    /// Read an artifact's metadata file. `StoreError::NotFound` where no card has
    /// been written for that hash — which is an ordinary state, not a damaged
    /// store: metadata is derived, and an artifact put by an older build has none
    /// until something regenerates it.
    pub fn read_meta(&self, hash: &Hash) -> Result<Vec<Line>, StoreError> {
        let path = self.meta_path(hash);
        if !path.exists() {
            return Err(StoreError::NotFound(*hash));
        }
        ndjson::read(&path)
    }

    /// Read a Set file (`sets/<id>.kbset`).
    pub fn read_set(&self, id: &str) -> Result<Vec<Line>, StoreError> {
        ndjson::read(&self.set_path(id))
    }

    /// Atomically writes a Set file (`sets/<id>.kbset`), validating that no metadata, authoring, or tick records are present.
    pub fn write_set(&self, id: &str, lines: &[Line]) -> Result<(), StoreError> {
        refuse_what_is_not_a_set(lines)?;
        ndjson::write(&self.set_path(id), lines)
    }

    /// Atomically writes a Set file to the sandbox directory (`sandbox/<id>.kbset`).
    pub fn write_sandbox_set(&self, id: &str, lines: &[Line]) -> Result<(), StoreError> {
        refuse_what_is_not_a_set(lines)?;
        ndjson::write(&self.sandbox_path(id), lines)
    }

    /// Read a session stream (`sessions/<stamp>.ndjson`).
    pub fn read_session(&self, stamp: &str) -> Result<Vec<Line>, StoreError> {
        ndjson::read(&self.session_path(stamp))
    }

    /// Write a session stream. Unlike [`Store::write_set`], ticks are expected here
    /// — a session is the timeline, ticks and all.
    pub fn write_session(&self, stamp: &str, lines: &[Line]) -> Result<(), StoreError> {
        ndjson::write(&self.session_path(stamp), lines)
    }

    /// Opens a session stream file for appending incoming timeline records.
    pub fn append_session(&self, stamp: &str) -> Result<fs::File, StoreError> {
        Ok(fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.session_path(stamp))?)
    }

    /// Save a live session as a Set: the session stream with ticks dropped and the
    /// state folded down, last write wins per layer and key. See
    /// [`project::project`] and `docs/ir-spec.md`, Session stream format.
    pub fn save_session_as_set(&self, set_id: &str, session: &[Line]) -> Result<(), StoreError> {
        self.write_set(set_id, &project::project(session))
    }

    /// Atomically writes a serialized console arrangement JSON document (`arrangements/<name>.arrangement.json`).
    pub fn write_arrangement(&self, name: &str, arrangement: &[u8]) -> Result<(), StoreError> {
        ndjson::write_atomic(&self.arrangement_path(name), arrangement)
    }

    /// Reads raw bytes of a saved console arrangement by name.
    ///
    /// Returns [`StoreError::NoArrangement`] if the file does not exist.
    /// Does not fall back to built-in presets (Principle 0096).
    pub fn read_arrangement(&self, name: &str) -> Result<Vec<u8>, StoreError> {
        fs::read(self.arrangement_path(name)).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => StoreError::NoArrangement(name.to_string()),
            _ => StoreError::Io(e),
        })
    }

    /// Lists stored arrangements in ascending name order with modification timestamps.
    ///
    /// Ignores non-matching files and directories. File content is not parsed.
    pub fn list_arrangements(&self) -> Result<Vec<ArrangementEntry>, StoreError> {
        let mut out = Vec::new();
        for entry in fs::read_dir(self.root.join("arrangements"))? {
            let entry = entry?;
            let file_name = entry.file_name();
            let Some(name) = file_name
                .to_str()
                .and_then(|n| n.strip_suffix(".arrangement.json"))
            else {
                continue;
            };
            if entry.file_type()?.is_dir() {
                continue;
            }
            out.push(ArrangementEntry {
                name: name.to_string(),
                written: entry.metadata()?.modified()?,
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Lists stored procedures in ascending name order with modification timestamps.
    ///
    /// Ignores non-matching files and directories. File content is not parsed.
    pub fn list_procedures(&self) -> Result<Vec<ProcedureEntry>, StoreError> {
        let mut out = Vec::new();
        for entry in fs::read_dir(self.root.join(Store::PROCEDURES))? {
            let entry = entry?;
            let file_name = entry.file_name();
            let Some(name) = file_name
                .to_str()
                .and_then(|n| n.strip_suffix(Store::PROCEDURE_FILE_SUFFIX))
            else {
                continue;
            };
            if entry.file_type()?.is_dir() {
                continue;
            }
            out.push(ProcedureEntry {
                name: name.to_string(),
                written: entry.metadata()?.modified()?,
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Reads a stored procedure's raw bytes by name.
    ///
    /// Returns [`StoreError::NoProcedure`] if no procedure exists with the given name.
    pub fn read_procedure(&self, name: &str) -> Result<Vec<u8>, StoreError> {
        fs::read(self.procedure_path(name)).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => StoreError::NoProcedure(name.to_string()),
            _ => StoreError::Io(e),
        })
    }

    /// Writes a procedure file to `procedures/<name>.kir`.
    ///
    /// Returns [`StoreError::ProcedureTaken`] if a file with the given name already exists.
    pub fn write_procedure(&self, name: &str, source: &[u8]) -> Result<PathBuf, StoreError> {
        let path = self.procedure_path(name);
        if path.exists() {
            return Err(StoreError::ProcedureTaken(name.to_string()));
        }
        ndjson::write_atomic(&path, source)?;
        Ok(path)
    }

    /// Writes a procedure file to the sandbox directory `sandbox/<name>.kir`.
    ///
    /// Returns [`StoreError::ProcedureTaken`] if a file with the given name already exists.
    pub fn write_sandbox_procedure(
        &self,
        name: &str,
        source: &[u8],
    ) -> Result<PathBuf, StoreError> {
        let path = self
            .root
            .join(Store::SANDBOX)
            .join(format!("{name}{}", Store::PROCEDURE_FILE_SUFFIX));
        if path.exists() {
            return Err(StoreError::ProcedureTaken(name.to_string()));
        }
        ndjson::write_atomic(&path, source)?;
        Ok(path)
    }

    /// Lists stored Sets sorted in ascending order by ID with filesystem modification timestamps.
    ///
    /// Only files ending in [`SET_FILE_SUFFIX`](Store::SET_FILE_SUFFIX) are included;
    /// non-matching entries and temporary files are ignored.
    pub fn list_sets(&self) -> Result<Vec<SetEntry>, StoreError> {
        let mut out = Vec::new();
        for entry in fs::read_dir(self.root.join("sets"))? {
            let entry = entry?;
            let name = entry.file_name();
            // Non-UTF-8 fails `to_str` and falls out of the listing with
            // everything else the layout does not claim — no lossy repair, and
            // no unwrap for a hostile name to trip.
            let Some(id) = name
                .to_str()
                .and_then(|n| n.strip_suffix(Store::SET_FILE_SUFFIX))
            else {
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

    /// The filename under the store root where favourite Set IDs are kept.
    pub const FAVOURITES_FILE: &str = "favourites.json";

    fn favourites_path(&self) -> PathBuf {
        self.root.join(Store::FAVOURITES_FILE)
    }

    /// Returns the set of starred Set IDs.
    ///
    /// If the favourites file does not exist, returns an empty set.
    /// Returns [`StoreError::Favourites`] if the file cannot be parsed.
    pub fn favourites(&self) -> Result<BTreeSet<String>, StoreError> {
        let bytes = match fs::read(self.favourites_path()) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
            Err(e) => return Err(StoreError::Io(e)),
        };
        let ids: Vec<String> =
            serde_json::from_slice(&bytes).map_err(|source| StoreError::Favourites { source })?;
        Ok(ids.into_iter().collect())
    }

    /// Sets or clears the favourite status of a Set.
    ///
    /// Returns `Ok(true)` if the state changed, or `Ok(false)` if the state was already set.
    /// Returns [`StoreError::NoSet`] if attempting to star a Set that does not exist.
    pub fn set_favourite(&self, id: &str, favourite: bool) -> Result<bool, StoreError> {
        if favourite && !self.set_path(id).is_file() {
            return Err(StoreError::NoSet(id.to_string()));
        }
        let mut ids = self.favourites()?;
        let moved = if favourite {
            ids.insert(id.to_string())
        } else {
            ids.remove(id)
        };
        if !moved {
            return Ok(false);
        }
        let ids: Vec<&String> = ids.iter().collect();
        let bytes = serde_json::to_vec(&ids).expect("a list of strings serialises");
        ndjson::write_atomic(&self.favourites_path(), &bytes)?;
        Ok(true)
    }

    /// The name of the file under the store root where per-slot MCP policies are kept.
    pub const POLICIES_FILE: &str = "policies.json";

    fn policies_path(&self) -> PathBuf {
        self.root.join(Store::POLICIES_FILE)
    }

    /// The per-slot MCP policies stored on disk.
    ///
    /// A missing file answers with an empty list.
    pub fn policies(&self) -> Result<Vec<String>, StoreError> {
        let bytes = match fs::read(self.policies_path()) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(StoreError::Io(e)),
        };
        let names: Vec<String> =
            serde_json::from_slice(&bytes).map_err(|source| StoreError::Policies { source })?;
        Ok(names)
    }

    /// Write the per-slot MCP policies to `<store>/policies.json`.
    pub fn write_policies<S: AsRef<str>>(&self, policies: &[S]) -> Result<(), StoreError> {
        let names: Vec<&str> = policies.iter().map(|s| s.as_ref()).collect();
        let bytes = serde_json::to_vec_pretty(&names).expect("a list of strings serialises");
        ndjson::write_atomic(&self.policies_path(), &bytes)?;
        Ok(())
    }

    /// Lists stored artifacts in ascending hash order, indicating whether each has metadata.
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

/// Parses a hash stem into a [`Hash`], returning `None` if the stem is invalid or not canonical hex.
fn parse_hash_stem(stem: &str) -> Option<Hash> {
    let hash: Hash = format!("sha256:{stem}").parse().ok()?;
    (hash.short(64) == stem).then_some(hash)
}
