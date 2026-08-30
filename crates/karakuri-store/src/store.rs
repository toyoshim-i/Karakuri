//! The on-disk artifact store.
//!
//! Layout under the store root (`library/` by default):
//!
//! ```text
//! <hash>.kir                        source, immutable
//! <hash>.meta.ndjson                regenerated metadata
//! thumbnails/<hash>.mp4
//! sets/<id>.kbset
//! sessions/<stamp>.ndjson
//! arrangements/<name>.arrangement.json
//! ```
//!
//! `<hash>` is the artifact's content address rendered as bare lowercase
//! hex (no `sha256:` prefix and no colon) so it is safe as a path component
//! on every platform the store might run on.
//!
//! **`arrangements/` is the fourth thing here and it is the operator's own.**
//! An artifact is material, a Set is a projection of material, a session is a
//! timeline of material — and an arrangement is none of those: it is the shape
//! of the console the operator plays them on, with no artifact in it. It is
//! filed under a **name the operator picked**, because a name is the one handle
//! that does not move when something unrelated moves
//! (`docs/principles/0053-a-value-that-must-be-stable-is-recorded-not-derived.md`),
//! and the name rule is a Set id's rule for the same reason it is a Set id's:
//! it becomes one path component. See
//! `docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md`.
//!
//! **What this module does not do is parse one.** The bytes are handed over
//! whole, exactly as `<hash>.kir` source is: the format belongs to
//! `karakuri-layout`, whose loader refuses an arrangement that disagrees with
//! itself rather than repairing it
//! (`docs/adr/0158-a-saved-arrangement-that-disagrees-with-itself-is-refused-not-repaired.md`),
//! and a check here would be a second answer to a question that already has
//! one.

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
    /// No arrangement is filed under that name.
    ///
    /// A variant of its own rather than [`StoreError::NotFound`], which carries
    /// a [`Hash`] and could not say this: an arrangement is addressed by a name
    /// somebody typed, and the sentence an operator needs is the name back.
    #[error("no arrangement named `{0}`")]
    NoArrangement(String),
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
    /// **A `part` record, which is the authoring form's way of naming a node**
    /// — by relative path, resolved against the Set file's own directory — in a
    /// file that is about to be written under `sets/` as a `.kbset`.
    ///
    /// A third sentence for a third reason, on [`StoreError::MetaInSet`]'s
    /// terms: the check is `Record::is_authoring`, and running it under either
    /// of the other two messages would tell an operator their authoring file
    /// carries time, or declares an artifact. It does neither. It says exactly
    /// what a `slot` says and has not been resolved yet, and the fix is to
    /// resolve it — `karakuri_environment::setfile`'s `resolve`, or
    /// `karakuri-cli --package FILE.kset` (or `--take-in FILE.kset`, which
    /// resolves it the same way and keeps the result).
    ///
    /// **What this makes structural is the extension's promise.** `.kbset`
    /// asserts that reading the file resolves nothing against the filesystem
    /// around it, which is what lets a swap be a swap
    /// (`docs/adr/0231-…`); a `part` written into `sets/` would be a file
    /// disagreeing with its own name, and the disagreement would surface on a
    /// frame boundary rather than here.
    #[error(
        "set files cannot contain a part record — a `part` names its `.kir` by relative path \
         and belongs to the authoring form (`.kset`), where `sets/` holds the resolved form \
         (`.kbset`) and only that; resolve it first (offending record at index {index})"
    )]
    PartInSet { index: usize },
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
        // Established on `open` like the other three, so that a store written
        // by an older build gains the directory the first time this one opens
        // it and `list_arrangements` answers "nothing kept" rather than
        // "no such directory".
        fs::create_dir_all(root.join("arrangements"))?;
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

    /// **What a stored Set file is called**, and the one place the suffix is
    /// spelled. `Store::set_path` builds the name and [`Store::list_sets`]
    /// derives an id back off it **by stripping this**, so two literals could
    /// drift into a store that writes files it cannot list — and an id that
    /// exists on disk under one spelling and nowhere in the listing is the
    /// worst shape that disagreement can take, because neither side is wrong
    /// on its own.
    ///
    /// **`.kbset` rather than `.set.ndjson`, and the reason is atomicity
    /// rather than tidiness.** A Set file has two forms: an authoring one,
    /// which names its parts by **relative path** and lives beside them, and
    /// this one, which names them by content address and needs nothing from the
    /// filesystem around it. A swap happens on a frame boundary and an
    /// over-budget Set rolls back on its own
    /// (`docs/principles/0005-a-swap-happens-on-a-frame-boundary-and-an-over-budget-set-rolls-back-on-its-own.md`),
    /// and that holds only because **nothing is left to resolve at the moment
    /// of the swap**. Let the authoring form into the store and a load walks
    /// the filesystem while swapping: a neighbour may be missing, may have
    /// changed since the file was written, may fail halfway — and a swap that
    /// can partially fail is not a swap. So the store's invariant is that
    /// everything in it is already resolved, and **the extension is what makes
    /// that invariant checkable**.
    ///
    /// **This is not a new check.** The suffix was always stripped to find an
    /// id, so an id could never exist without it; what changed is that the
    /// check now means something. The authoring form's own spelling is
    /// `.kset`, and it is deliberately not a constant *here*: it is
    /// `karakuri_environment::setfile::AUTHORING_SUFFIX`, beside the resolver
    /// that is the only thing which reads one. This crate never opens a `.kset`
    /// and could not — a store that held one would be the thing the suffix
    /// above exists to make impossible — so a name for it here would be this
    /// module describing a file it has no business with. (Until the resolver
    /// landed there was no constant anywhere, and this comment said so; a name
    /// nothing reads is a claim about a design rather than part of one.) See
    /// `docs/adr/0231-a-sets-two-forms-take-two-extensions-and-the-store-holds-only-the-resolved-one.md`.
    pub const SET_FILE_SUFFIX: &str = ".kbset";

    fn set_path(&self, id: &str) -> PathBuf {
        self.root
            .join("sets")
            .join(format!("{id}{}", Store::SET_FILE_SUFFIX))
    }

    fn session_path(&self, stamp: &str) -> PathBuf {
        self.root.join("sessions").join(format!("{stamp}.ndjson"))
    }

    /// **`<name>.arrangement.json`, ending in a suffix the layout owns the way
    /// `<id>.kbset` does**: the operator's name, what kind of thing it is, and
    /// the format it is in. That suffix is what lets
    /// [`Store::list_arrangements`] tell an arrangement from an editor's backup
    /// or a half-written `.tmp` without opening either, which is the same trick
    /// `sets/` turns with [`Store::SET_FILE_SUFFIX`] — there in one component
    /// rather than two, because a Set's extension has a second job this one
    /// does not: it says the file is already resolved.
    ///
    /// **`.json` and not `.ndjson`.** An arrangement is one document rather
    /// than a stream of records — there is no line to append and nothing to
    /// project — so it does not go through [`crate::ndjson`]'s reader at all.
    fn arrangement_path(&self, name: &str) -> PathBuf {
        self.root
            .join("arrangements")
            .join(format!("{name}.arrangement.json"))
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

    /// Read a Set file (`sets/<id>.kbset`).
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
    ///
    /// **And rejects a `part`, which is where the two forms of a Set file are
    /// held apart.** `sets/<id>.kbset` asserts that everything in it is already
    /// resolved; a `part` names its `.kir` by a relative path and is the
    /// authoring form's record, so this is the wall between a `.kset` and a
    /// store. Three questions now, one classification, and a third sentence
    /// because a third thing is owed — see
    /// [`Record::is_authoring`](crate::record::Record::is_authoring) and
    /// [`StoreError::PartInSet`].
    pub fn write_set(&self, id: &str, lines: &[Line]) -> Result<(), StoreError> {
        if let Some(index) = lines.iter().position(|l| l.record().is_metadata()) {
            return Err(StoreError::MetaInSet { index });
        }
        // Before the `is_set_state` scan below, which would also refuse it —
        // and would name a tick while doing so. Asked first for the reason
        // `is_metadata` is asked first: the earlier question owns the more
        // specific sentence.
        if let Some(index) = lines.iter().position(|l| l.record().is_authoring()) {
            return Err(StoreError::PartInSet { index });
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

    /// **Write a saved arrangement** (`arrangements/<name>.arrangement.json`).
    ///
    /// The bytes are `karakuri-layout`'s, not this crate's: a `Layout`
    /// serialises to one JSON document and that document is what lands here,
    /// byte for byte and with nothing appended. Handing it over whole is the
    /// same contract [`Store::put_artifact`] has with `.kir` source, and for
    /// the same reason — the store keeps files it does not have to understand,
    /// and the one place that understands this format is the loader that
    /// refuses a broken one (ADR-0158).
    ///
    /// **Overwrites, like [`Store::write_set`] and unlike
    /// [`Store::put_artifact`].** A name is an instruction: saving over
    /// `four_deck` is what an operator who has just moved a divider means by
    /// saving `four_deck`, and `--save-set ID` has always obeyed the same way.
    /// Atomic all the same, so a reader never meets half a document, and a
    /// crash mid-write leaves the previous arrangement rather than nothing.
    ///
    /// **Nothing here checks the name**, exactly as nothing checks a Set id:
    /// `<name>` becomes one path component and that is the caller's rule to
    /// keep. Where the caller is a protocol rather than a person it is kept —
    /// `karakuri-environment`'s `mcp::checked_id` is where a name that is not
    /// one path component is refused rather than sanitised, and an arrangement
    /// name reached from a model belongs behind the same gate.
    pub fn write_arrangement(&self, name: &str, arrangement: &[u8]) -> Result<(), StoreError> {
        ndjson::write_atomic(&self.arrangement_path(name), arrangement)
    }

    /// **Read a saved arrangement back**, as the bytes that were written.
    ///
    /// `StoreError::NoArrangement` where nothing is filed under that name,
    /// which is an ordinary answer rather than a damaged store: an operator
    /// asking for an arrangement they have not saved is a person to tell, and
    /// the name they asked for is what the sentence carries.
    ///
    /// **This never returns the built-in.** Resetting the console is a
    /// different operation reaching different code — the default arrangement is
    /// what ships, and what ships is not a file anything here can write
    /// ([P-0048](../../../docs/principles/0048-what-ships-what-you-saved-and-what-you-are-editing-are-three-places.md)).
    /// A fallback here would make an operator who mistyped a name watch their
    /// console reset instead of being told the name is wrong.
    pub fn read_arrangement(&self, name: &str) -> Result<Vec<u8>, StoreError> {
        fs::read(self.arrangement_path(name)).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => StoreError::NoArrangement(name.to_string()),
            _ => StoreError::Io(e),
        })
    }

    /// **List the arrangements the store holds**, in ascending name order, each
    /// with the time its file was last written.
    ///
    /// Everything [`Store::list_sets`] says about its listing holds here and
    /// for the same reasons: the time comes from the filesystem because the
    /// document carries none; the order is the name's rather than recency's,
    /// because two files written inside one tick of a coarse clock tie and a
    /// tied sort is not an order; a name the layout does not claim is skipped
    /// rather than repaired, so an editor's backup and a `.tmp` left by a write
    /// that died are not offered as arrangements [`Store::read_arrangement`]
    /// cannot open; and an empty store lists nothing while a missing directory
    /// is an error.
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
    /// directory holds `<id>`[`SET_FILE_SUFFIX`](Store::SET_FILE_SUFFIX); an
    /// editor's backup, a `.tmp` left by a write that died, a subdirectory
    /// someone made — those belong to whoever put them there, and reporting one
    /// as a Set under a truncated id would invent a library entry
    /// [`Store::read_set`] cannot open.
    ///
    /// **So a file under `sets/` that does not carry that suffix has no id at
    /// all**, and an id is the only route a Set has to a deck: nothing can ask
    /// for what cannot be named. That is what keeps a swap atomic
    /// (`docs/principles/0005-a-swap-happens-on-a-frame-boundary-and-an-over-budget-set-rolls-back-on-its-own.md`) —
    /// the authoring form of a Set names its parts by relative path, so loading
    /// one would resolve against the filesystem mid-swap, and a swap that can
    /// partially fail is not a swap. The listing is where that wall stands,
    /// and it stands by naming rather than by opening anything.
    ///
    /// **The concrete cost, because it is paid silently.** A Set written by a
    /// build that spelled the suffix `.set.ndjson` is still on disk, still
    /// readable text, and simply **stops appearing here** — no error, no
    /// warning, nothing to notice but an id that used to be in the list and is
    /// not. Nothing repairs it and nothing should: renaming a file this store
    /// did not write would be guessing that its contents are already resolved,
    /// which is the one thing the extension exists to stop being a guess.
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

/// An arrangement the store holds, as [`Store::list_arrangements`] found it:
/// what to hand [`Store::read_arrangement`], and when that file was last
/// written.
///
/// **`name` rather than `id`**, where [`SetEntry`] says `id`. A Set is
/// ordinarily filed under a stamp nobody chose — `history::stamped_id`, because
/// a key press cannot type a name — and an arrangement never is: it is saved by
/// an operator who is telling the console what to call this shape. The two
/// words are the difference, and carrying `id` here would say a stamp is the
/// expected case when it is the fallback.
///
/// The time is a field for the reason it is one on [`SetEntry`]: a name an
/// operator typed sorts nowhere near when they typed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrangementEntry {
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
