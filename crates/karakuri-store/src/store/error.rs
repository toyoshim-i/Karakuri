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
    /// No arrangement is filed under that name.
    ///
    /// A variant of its own rather than [`StoreError::NotFound`], which carries a
    /// [`Hash`] and could not say this: an arrangement is addressed by a name
    /// somebody typed, and the sentence an operator needs is the name back.
    #[error("no arrangement named `{0}`")]
    NoArrangement(String),
    /// No procedure is filed under that name.
    ///
    /// [`StoreError::NoArrangement`]'s sibling and for its reason: a procedure is
    /// addressed by a name somebody typed, and the sentence whoever asked needs
    /// back is that name
    /// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
    /// A row of the Library bay that has gone since the listing was built is the
    /// ordinary way to meet it, and it is exactly the sentence a load has to say
    /// rather than swallow.
    #[error("no procedure named `{0}`")]
    NoProcedure(String),
    /// A keep was asked for under a name `procedures/` already holds.
    ///
    /// Refused rather than overwritten, where an arrangement and a Set id are not,
    /// and the difference is what each name is over. An arrangement's name is a
    /// *place an operator keeps coming back to* — saving `four_deck` again is what
    /// an operator who has just moved a divider means (ADR-0221) — and a Set id
    /// typed twice replaces what is under it because the caller typed it
    /// (ADR-0128). A kept procedure is neither: the name arrives from a capsule
    /// that types nothing and takes a stamp, or from a head that typed one once,
    /// and what is under it is somebody's part of a library that a later keep of a
    /// different node would silently replace.
    ///
    /// The sentence carries the name back, which is the whole of what the next
    /// attempt needs
    /// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)):
    /// there is one thing to change and it is the name.
    #[error(
        "a procedure named `{0}` is already kept — a keep never overwrites one, so name this \
         one something else"
    )]
    ProcedureTaken(String),
    /// A star was asked for on an id `sets/` does not hold.
    ///
    /// Its own variant beside [`StoreError::NoArrangement`] and for that variant's
    /// reason: [`StoreError::NotFound`] carries a [`Hash`] and could not say this,
    /// and what an operator needs back is the id they named
    /// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
    ///
    /// Only starring is refused. Taking a star *off* an id the store no longer
    /// holds is how a stale mark is cleared, so [`Store::set_favourite`] asks this
    /// question in one direction only.
    #[error("no Set named `{0}` in this store, so there is nothing to star")]
    NoSet(String),
    /// The favourites file is on disk and is not a list of ids.
    ///
    /// Said rather than swallowed: a file somebody hand-edited into something
    /// unparseable would otherwise read exactly like a library nobody has starred
    /// in, and the whole of `my sets` would go quiet with nothing to notice. A
    /// missing file is *not* this — that is a store nobody has starred in, and
    /// [`Store::favourites`] answers it with an empty set.
    #[error("`{}` is not a list of Set ids: {source}", Store::FAVOURITES_FILE)]
    Favourites {
        #[source]
        source: serde_json::Error,
    },
    /// The slot policies file is on disk and is not valid.
    #[error("`{}` is not valid slot policies: {source}", Store::POLICIES_FILE)]
    Policies {
        #[source]
        source: serde_json::Error,
    },
    /// A Set file carries no time — see `docs/ir-spec.md`, Set file format. `tick`
    /// was the only such record when this was named; `audio` and `tempo` are the
    /// same kind of thing, so the check is `Record::is_set_state` and the message
    /// says so rather than naming one of the three.
    #[error(
        "set files cannot contain a tick, audio or tempo record — a Set file carries no time \
         (offending record at index {index})"
    )]
    TickInSet { index: usize },
    /// A Set file says what a Set's values *are*; a `meta`, `param_decl`,
    /// `capacity_decl` or `emit` record says what an artifact *declares*, and
    /// belongs in `<hash>.meta.ndjson`. Refused on the same terms as a session
    /// record and with a different sentence, because the reason differs: the check
    /// is `Record::is_metadata` rather than `!Record::is_set_state`, and running
    /// the two together under one message would tell an operator a declaration was
    /// rejected for carrying time.
    #[error(
        "set files cannot contain a meta, param_decl, capacity_decl or emit record — those \
         describe what an artifact declares and belong in its `<hash>.meta.ndjson` \
         (offending record at index {index})"
    )]
    MetaInSet { index: usize },
    /// A `part` record, which is the authoring form's way of naming a node — by
    /// relative path, resolved against the Set file's own directory — in a file
    /// that is about to be written under `sets/` as a `.kbset`.
    ///
    /// A third sentence for a third reason, on [`StoreError::MetaInSet`]'s terms:
    /// the check is `Record::is_authoring`, and running it under either of the
    /// other two messages would tell an operator their authoring file carries time,
    /// or declares an artifact. It does neither. It says exactly what a `slot` says
    /// and has not been resolved yet, and the fix is to resolve it —
    /// `karakuri_environment::setfile`'s `resolve`, or `karakuri-cli --package
    /// FILE.kset` (or `--take-in FILE.kset`, which resolves it the same way and
    /// keeps the result).
    ///
    /// What this makes structural is the extension's promise. `.kbset` asserts that
    /// reading the file resolves nothing against the filesystem around it, which is
    /// what lets a swap be a swap (`docs/adr/0231-…`); a `part` written into
    /// `sets/` would be a file disagreeing with its own name, and the disagreement
    /// would surface on a frame boundary rather than here.
    #[error(
        "set files cannot contain a part record — a `part` names its `.kir` by relative path \
         and belongs to the authoring form (`.kset`), where `sets/` holds the resolved form \
         (`.kbset`) and only that; resolve it first (offending record at index {index})"
    )]
    PartInSet { index: usize },
}
