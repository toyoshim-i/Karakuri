#[allow(unused_imports)]
use super::types::*;
use super::variants::*;

/// Which of the three files' vocabularies a record belongs to, decided in one
/// exhaustive match that both of the questions below are read off.
///
/// One classification and two questions, because the two drifted apart once.
/// [`Record::is_set_state`] and [`Record::is_metadata`] ask different things —
/// *may this go in a Set file* and *does this belong in a card* — and each owes
/// an operator a different sentence, which is why there are two of them. What
/// there is not is two answers: the four metadata variants were added to the
/// enum and classified in neither function, and because `is_set_state` was a
/// `matches!` over the exceptions it answered *true* for all four, under a
/// headline saying they belong in a Set file.
///
/// Two exhaustive matches would have caught that and would still leave the next
/// record classifiable one way here and another way there. Classified once,
/// they cannot disagree, and a variant with no arm does not compile — `origin`,
/// `parent`, `perf`, `tag` and `thumbnail` are specified and will arrive.
enum Vocabulary {
    /// What a Set *is*: written to a Set file and read back out of one.
    Set,
    /// Not state at all — what a *frame* saw or decided.
    Frame,
    /// The session's state rather than any Set's: the deck the Sets are playing on.
    Session,
    /// What an *artifact* declares, in its `<hash>.meta.ndjson`.
    Metadata,
    /// What a Set is *before it is resolved*: the authoring form, `.kset`, which
    /// names its `.kir` files by relative path and lives beside them.
    ///
    /// A vocabulary of its own and not a corner of [`Vocabulary::Set`], though what
    /// it says is a Set's own state and nothing else's. The question every reader
    /// here asks is *which file may this line be in*, and the answer for a
    /// [`Record::Part`] is one file the store may never hold: `<store>/sets/` is
    /// `.kbset` and only `.kbset`, because reading a `.kbset` resolves nothing
    /// against the filesystem and that is what makes a swap unable to half-fail.
    /// Classified `Set`, it would be written into a store by the one function that
    /// exists to stop that (`Store::write_set`), and the file would then be a
    /// `.kbset` naming a neighbour it may no longer have.
    Authoring,
    /// A `t` this build does not know. Not a vocabulary but the absence of one, and
    /// it is a case of its own because every reader treats it as a line to carry
    /// rather than a line to place: passing it over is the format's promise, so a
    /// Set file round-tripped through a build that does not know every record in it
    /// comes back with all of them.
    Unknown,
}

impl Record {
    /// Whether this record belongs in a Set file. Twenty-three say no, for four
    /// different reasons, and keeping them apart is the point of the name — it is
    /// `is_set_state` rather than `is_state` because most of what it refuses is
    /// state. (It read "twenty-one" until [`Record::Part`] arrived and the variants
    /// were counted: eighteen, plus the four metadata records, plus this one. A
    /// prose count is not checked by anything, which the group comment above
    /// [`Record::Gain`] confesses to about its own two. The *classification* cannot
    /// drift the same way — see [`Vocabulary`].)
    ///
    /// - [`Record::Tick`], [`Record::Audio`] and [`Record::Tempo`] are not state at
    ///   all: they are what a *frame* saw or decided. A Set file carries no time, and
    ///   one holding an audio frame would be claiming a particular moment's
    ///   microphone reading is part of what a Set is.
    /// - [`Record::Gain`], [`Record::Opacity`], [`Record::Blend`], [`Record::Residency`],
    ///   [`Record::Look`], [`Record::Canvas`], [`Record::Procedure`],
    ///   [`Record::Authority`], [`Record::Source`], [`Record::Transport`],
    ///   [`Record::Mask`], [`Record::Transition`] and [`Record::Select`] are the
    ///   session's rather than any Set's. The last two are the ones that are not
    ///   durable state at all but *events* — a move and a choice, each scheduled at
    ///   an instant — and they are here rather than beside `tick` because what they
    ///   move is the deck. Folding a session down to a Set file drops them for both
    ///   reasons at once. A Set does not know what fader it is under; one that
    ///   carried its gain would restore that gain wherever it was next loaded, which
    ///   is a Set file reaching outside the Set.
    ///
    /// [`Record::Canvas`] is in this group for a reason worth stating apart: a Set
    /// renders at whatever size it is given, and one that carried a canvas would
    /// make loading it resize every *other* Set in the deck.
    ///
    /// [`Record::Save`] joins that second group: it says a deck slot's material was
    /// written out, which is a fact about a performance and about no Set. A Set
    /// file carrying one would claim, every time it was loaded, that a save had
    /// just happened.
    ///
    /// [`Record::Select`] says which renderer of a deck slot is live, which is a
    /// fact about a run — and it used to be in this group twice over, because the
    /// layering that makes the question mean anything was not in a Set file at all.
    /// It is now: [`Record::Merge`] records it, and its `live` field is where a
    /// *Set* says which renderer is the live one. What remains is the first reason
    /// and it is enough — a selection is addressed to a deck slot and scheduled at
    /// an instant, and no session record says which deck slot's Set composites or
    /// which deck slot a folded Set was played in.
    ///
    /// - [`Record::Meta`], [`Record::ParamDecl`], [`Record::CapacityDecl`] and
    ///   [`Record::Emit`] are a third file's vocabulary: what an artifact *declares*,
    ///   before anything has instantiated it. Not the session's and not any Set's,
    ///   which is why they are a third reason rather than a longer second one.
    /// - [`Record::Part`] is a fourth reason and the only one that is not about
    ///   what the record says. It is this Set's own state, in the vocabulary of the
    ///   *authoring* form: a node named by relative path rather than by content
    ///   address. Nothing is wrong with what it says — it says what a
    ///   [`Record::Slot`] says — and everything is wrong with where it is, because
    ///   `<store>/sets/` holds `.kbset` and only `.kbset` so that a swap has nothing
    ///   left to resolve. Refused through [`Record::is_authoring`], for the reason a
    ///   `param_decl` is refused through [`Record::is_metadata`]: a third sentence is
    ///   owed, and it is about which of a Set's two forms the file is.
    ///
    /// Four reasons for one answer, and `Record::Save` still does not add another.
    /// The question here is *may this line go in a Set file*, and it has one answer
    /// per record however many reasons stand behind a no. Whether a record reaches
    /// outside the stream is a different question about the same vocabulary, and
    /// `Record::Save` is so far the only record for which the answer is yes;
    /// folding that in would give one function two jobs. It lives in
    /// `docs/ir-spec.md` under "Records with an effect outside the stream", where a
    /// replay reads it.
    ///
    /// A session stream carries the eighteen of the first two groups and none of
    /// the third or the fourth. That is the difference between the two files,
    /// stated from this side, and it is what [`Record::is_metadata`] is a separate
    /// function for: `Store::write_set` refuses a `param_decl` through *that*
    /// question so that the sentence it prints is about declarations.
    /// `!is_set_state()` refuses it too — the third bullet is what that means — but
    /// the refusal it reaches is the one naming a tick, and telling an operator a
    /// `capacity_decl` was rejected for carrying time sends them looking in the
    /// wrong place. Two questions, two sentences, one classification.
    ///
    /// What this said before, because an inverted reason outlives the code it was
    /// written about. The four metadata variants were added to the enum and not to
    /// this function, which was a `matches!` over the exceptions — so
    /// `is_set_state()` answered *true* for all four, under a headline that says
    /// they belong in a Set file. The paragraph here claimed a bare
    /// `!is_set_state()` "would refuse them for the wrong reason": it would not
    /// have refused them at all, it would have written them to disk, and only
    /// `Store::write_set` asking [`Record::is_metadata`] first kept that from
    /// happening. Neither question is answered by hand any more — both are read off
    /// [`Vocabulary`], whose match is exhaustive — so the next record cannot arrive
    /// the same way, and cannot be classified one way here and another way there.
    /// `origin`, `parent`, `perf`, `tag` and `thumbnail` are specified and will
    /// arrive; `project::key_for` and `setfile::from_lines` already stop compiling
    /// until somebody classifies them, and this does too now.
    ///
    /// That is a claim about the readers that have to place a record, and not about
    /// every match on `Record` in the workspace. `karakuri-cli`'s `mix::change`
    /// ends with `_ => Ok(None)`, so a new variant silently becomes "not a mix
    /// change" there. It is deliberate and it is the safe default — that function's
    /// whole contract is that a record it does not act on is not an error, so that
    /// a stream from a newer build replays rather than failing — but it means a new
    /// *deck* record can arrive, be classified `Session` here, and still go unacted
    /// on with nothing saying so. The exhaustive matches are the ones that decide
    /// which file a record belongs in; the wildcard is in the one that decides what
    /// to do with it.
    ///
    /// [`Record::Unknown`] is the one `true` that is not a claim about state. An
    /// unrecognised `t` is passed over rather than refused, which is the format's
    /// promise and the reason `project::key_for` gives one its own passthrough key.
    /// Answering false for it would make `Store::write_set` reject a file it had
    /// just read.
    pub fn is_set_state(&self) -> bool {
        matches!(self.vocabulary(), Vocabulary::Set | Vocabulary::Unknown)
    }

    /// Whether this record is a frame's own — what one frame saw or decided, rather
    /// than state something holds between frames. Three say yes: [`Record::Tick`],
    /// [`Record::Audio`] and [`Record::Tempo`].
    ///
    /// A fourth question for a fourth sentence, and the one `session::split` asks
    /// to decide where a record written before the first `tick` belongs. A stream's
    /// head is everything before the first tick *except* these: a frame writes its
    /// edits, then its measurement, then the tick that closes it, so the `audio`
    /// line in front of the first tick is the first frame's measurement and moving
    /// it into the head would replay frame 0 at what no microphone heard.
    ///
    /// Read off [`Vocabulary`] like the other three, so a new variant with no arm
    /// does not compile.
    pub fn is_measurement(&self) -> bool {
        matches!(self.vocabulary(), Vocabulary::Frame)
    }

    /// Which file's vocabulary this record is part of. The one place any record is
    /// classified — see [`Vocabulary`] for why it is one place.
    fn vocabulary(&self) -> Vocabulary {
        match self {
            // Written to a Set file and read back out of one, and this is the
            // arm that grows when the Set vocabulary does.
            Record::Header { .. }
            | Record::Set { .. }
            | Record::Slot { .. }
            | Record::Capacity { .. }
            | Record::Param { .. }
            | Record::Bind { .. }
            | Record::Camera { .. }
            | Record::Merge { .. }
            | Record::Seed { .. }
            | Record::Edge { .. }
            | Record::Src { .. } => Vocabulary::Set,
            // **The one record that is a Set's own state and still not a Set
            // file's**, because the file it belongs to is the one a store may
            // not contain. See [`Vocabulary::Authoring`] and
            // [`Record::is_authoring`].
            Record::Part { .. } => Vocabulary::Authoring,
            Record::Tick { .. } | Record::Audio { .. } | Record::Tempo { .. } => Vocabulary::Frame,
            Record::Gain { .. }
            | Record::Opacity { .. }
            | Record::Mute { .. }
            | Record::Solo { .. }
            | Record::Online { .. }
            | Record::Blend { .. }
            | Record::Residency { .. }
            | Record::Policy { .. }
            | Record::Look { .. }
            | Record::MasterOut { .. }
            | Record::MasterChain(_)
            | Record::Canvas { .. }
            | Record::Procedure { .. }
            | Record::Authority { .. }
            | Record::Ride { .. }
            | Record::Source { .. }
            | Record::Transport { .. }
            | Record::Transition { .. }
            | Record::Select { .. }
            | Record::Mask { .. }
            | Record::Save { .. } => Vocabulary::Session,
            // The arm `is_set_state` was missing when it was a `matches!` over
            // the exceptions: these were added to the enum without being
            // classified anywhere, and nothing said so.
            Record::Meta { .. }
            | Record::ParamDecl { .. }
            | Record::CapacityDecl { .. }
            | Record::Emit { .. } => Vocabulary::Metadata,
            Record::Unknown => Vocabulary::Unknown,
        }
    }

    /// Whether this record belongs in an artifact's `<hash>.meta.ndjson`. Four say
    /// yes, and they are the four a compile pass can produce.
    ///
    /// A separate question from [`Record::is_set_state`], which asks *may this go
    /// in a Set file* and answers no to these as well. Two functions because two
    /// sentences are owed, not because the classification is in doubt:
    /// `Store::write_set` asks this one first so that a rejected `param_decl` is
    /// told it is a declaration, rather than told it carries time. One
    /// classification behind both, [`Vocabulary`], because the two drifted apart
    /// once already — see the head of [`Record::is_set_state`].
    ///
    /// The specification lists five more — `origin`, `parent`, `perf`, `tag` and
    /// `thumbnail` — and nothing produces any of them yet. When one arrives it
    /// joins [`Vocabulary::Metadata`]'s arm and this doc's count moves with it; it
    /// cannot be forgotten on the way, because a variant with no arm does not
    /// compile.
    ///
    /// `thumbnail` is the fifth name because `preview` was taken, by the deck's
    /// record for which slot was being auditioned. The specification called the
    /// library asset `preview` too, and one `t` cannot carry two shapes: the deck
    /// record's `slot` was an `Option`, so the specified line decoded as the deck
    /// record and lost its `path` without a word. The metadata name moved rather
    /// than the deck's, which was written into session streams (ADR-0134); the
    /// deck's record has since been retired outright (ADR-0240) and the name is
    /// free, but `thumbnail` is the word the library already used and it stays.
    /// Nothing here has a `Thumbnail` variant, because nothing renders one yet — it
    /// joins `origin`, `parent`, `perf` and `tag` on the list of records the
    /// specification describes and nothing writes.
    pub fn is_metadata(&self) -> bool {
        matches!(self.vocabulary(), Vocabulary::Metadata)
    }

    /// Whether this record belongs to the authoring form of a Set file (`.kset`)
    /// rather than to the resolved one. One says yes: [`Record::Part`].
    ///
    /// A third question for a third sentence, which is the reason there were two —
    /// [`Record::is_metadata`] exists so that a rejected `param_decl` is told it is
    /// a declaration rather than told it carries time, and a `part` needs the same
    /// courtesy for a reason further from either: it is neither a declaration nor
    /// time nor the deck's. It is this Set's own state, written in the form that
    /// names its parts by path, and what is wrong with it inside a store is only
    /// that nothing has resolved it yet. `StoreError::PartInSet` says that, and
    /// says which of the two forms the file is — where `TickInSet`'s sentence would
    /// send an operator looking for a `tick` they did not write.
    ///
    /// Read off [`Vocabulary`] like the other two, so the three cannot disagree
    /// about one record and a new variant with no arm does not compile.
    pub fn is_authoring(&self) -> bool {
        matches!(self.vocabulary(), Vocabulary::Authoring)
    }
}
