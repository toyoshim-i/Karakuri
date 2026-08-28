//! The Set-file projection: "a Set file is the session stream with ticks
//! dropped and the state folded down" (`docs/ir-spec.md`, Session stream
//! format).
//!
//! Folding is last-write-wins per address and key, where "key" depends on the
//! record type — a [`Record::Param`] is keyed by `(layer, index, key)`, a
//! [`Record::Capacity`], a [`Record::Seed`] and a [`Record::Camera`] by the
//! node they address, and so on. **What a record says about a
//! node is folded; which node it says it about is what it is folded by**, so a
//! [`Record::Slot`]'s `name` is on the value side of that line and its
//! `(layer, index)` is on the key side.
//!
//! The result keeps each key at the position of its
//! *first* occurrence in the session but with its *last* value — the same
//! semantics as repeatedly `.insert()`-ing into an ordered map. That keeps
//! the projection stable: appending one more edit to a session changes at
//! most the value at an existing position, or appends one new position, so
//! saving the same live session twice in a row produces near-identical Set
//! files even as the session grows.
//!
//! [`Record::Unknown`] cannot be folded by this scheme at all: the variant
//! carries no data, so two unrelated unknown record types are
//! indistinguishable once parsed, and folding them together would silently
//! merge records that share nothing but the store's ignorance of them. Each
//! unknown line is therefore treated as its own key and passes through
//! unfolded, in its original relative position.

use std::collections::HashMap;

use crate::hash::Hash;
use crate::ndjson::Line;
use crate::record::{Layer, Record};

#[derive(Clone, PartialEq, Eq, Hash)]
enum Key {
    Set,
    Slot(Layer, u32),
    Capacity(Layer, u32),
    Param(Layer, Option<u32>, String),
    Bind(Layer, Option<u32>, String),
    Camera(u32),
    /// **No index beside it, because a Set holds exactly one L5.** Every other
    /// key here carries whatever says *which* node a record is about; a
    /// `merge` record addresses the only node it could address, so the key is
    /// the record type and a second `merge` in a session is one L5 with a
    /// later answer.
    Merge,
    Seed(Layer, u32),
    /// A node and the *input slot* of it being bound — the two halves of what an edge
    /// is *about*, where the node it is bound *to* is what the edge says.
    Edge(String, String),
    Src(Hash, u32),
    /// An unfoldable line (currently only `Record::Unknown`), identified by
    /// its position in the input so it never coalesces with another.
    Passthrough(usize),
}

/// The fold key for a record, or `None` for one that does not belong in a Set
/// file. **Four kinds of `None`, and the second is the one that is easy to get
/// wrong.**
///
/// The first is a record that carries no state at all: `Record::Tick`, which
/// the spec guarantees never appears in a Set file, and `Record::Audio`, which
/// is the same kind of thing — what one frame measured, not what anything *is*.
/// Folding a session's audio down to its last frame would put one arbitrary
/// moment's microphone reading into a Set file and call it state. `Tempo` is a
/// correction, so an event, and the tempo it corrects is the session's.
///
/// The second is a record that **is** state and is not this Set's: `Gain`,
/// `Opacity`, `Blend`, `Residency`, `Look`, `Canvas`, `Transport`, `Preview`,
/// `Mask` and
/// `Transition` describe the deck the Sets are playing on. Dropping them is not "there is nothing to fold", it
/// is "there is something to fold and this is not the projection it folds into"
/// — a Set file that restored a gain would apply it to whatever deck slot it
/// was next loaded into, one that restored a residency would put a Set on air
/// by being opened, and one that restored a canvas would resize every *other*
/// Set in the deck. **The projection that would fold them — a session down to
/// the deck state it ends at — does not exist**, and nothing needs it: a
/// session is replayed from the top rather than resumed from its end.
///
/// The third is `Save`, which is neither: it is not state at all and it is not
/// the deck's, it is a fact about something *outside* the stream — a Set file
/// that already exists under an id of its own. There is nothing here to fold
/// into, because what it names is a whole file this function's output is one
/// of. See the arm itself, and "Records with an effect outside the stream" in
/// `docs/ir-spec.md`.
///
/// The fourth is a **third file's vocabulary**: `Meta`, `ParamDecl`,
/// `CapacityDecl` and `Emit` say what an *artifact* declares, which is neither
/// this Set's state nor the deck's nor a fact about a file this one writes. A
/// session stream is a performance and nothing here puts one in one, so meeting
/// one means a hand-edited stream — see the arm, and `Record::is_metadata`.
///
/// **This count is prose and nothing checks it**, which is how it went on
/// saying three after the fourth arm was written below — the same drift
/// `Record`'s own group comment confesses to for its "seven" and "twelve". What
/// cannot drift that way is the classification: this match carries no wildcard,
/// so a new record stops this function compiling until somebody gives it an
/// arm, and `Record::vocabulary` is exhaustive for the same reason.
fn key_for(record: &Record, ordinal: usize) -> Option<Key> {
    match record {
        Record::Set { .. } => Some(Key::Set),
        // Keyed by the node's **address and not by its name**: a `slot`
        // record says which node it is about with `(layer, index)`, and the
        // name is one of the things it says about it. So a session that named
        // a node and then renamed it folds to one node with its later name,
        // where a fold keyed by the name would keep both lines and describe
        // two nodes that never existed.
        Record::Slot { layer, index, .. } => Some(Key::Slot(*layer, *index)),
        // A node rather than a layer, so two geometries at two capacities
        // survive the fold as the two facts they are. Keyed by layer alone
        // they collapsed onto each other and the projection kept whichever
        // line came last — a Set file that resizes the wrong source.
        Record::Capacity { layer, index, .. } => Some(Key::Capacity(*layer, *index)),
        // Folded by the **address**, so a wildcard write and a write addressed
        // at one node are two facts rather than one overwriting the other —
        // which is what they are: "the Set's exposure" and "renderer 1's
        // exposure" can both be true, and the engine resolves the overlap.
        Record::Param {
            layer, index, key, ..
        } => Some(Key::Param(*layer, *index, key.clone())),
        Record::Bind {
            layer, index, key, ..
        } => Some(Key::Bind(*layer, *index, key.clone())),
        // By the node, so that two cameras described in one session survive
        // the fold as the two producers they are — the same correction
        // `capacity` and `seed` needed when a layer stopped holding one node.
        Record::Camera { index, .. } => Some(Key::Camera(*index)),
        // **One L5, so one key.** A Set composites or it does not, and the
        // last line to say so is what the Set ends up being — which is the
        // same last-write-wins every other address here gets, over an address
        // with nothing in it.
        Record::Merge { .. } => Some(Key::Merge),
        // The same, and it is what a per-source salt *is*: two sources folded
        // onto one seed is two geometries salted alike, which is the one thing
        // salting exists to prevent.
        Record::Seed { stream, index, .. } => Some(Key::Seed(*stream, *index)),
        // **By the input slot it fills and not by what fills it**, which is
        // the same reason a `slot` record folds by its address rather than by
        // its name: rebinding an input slot in a session is one input slot
        // with a later answer, and a
        // fold keyed by the far end would keep both and describe a node with
        // two inputs it never had.
        Record::Edge { node, slot, .. } => Some(Key::Edge(node.clone(), slot.clone())),
        Record::Src { hash, line, .. } => Some(Key::Src(*hash, *line)),
        Record::Tick { .. }
        | Record::Audio { .. }
        | Record::Tempo { .. }
        | Record::Gain { .. }
        | Record::Opacity { .. }
        | Record::Blend { .. }
        | Record::Residency { .. }
        | Record::Look { .. }
        | Record::Canvas { .. }
        | Record::Procedure { .. }
        // **A node's address and still dropped**, which none of its neighbours
        // here are. An authority names `(layer, index)` the way a `slot` does,
        // so it looks foldable — and what it says is not about the Set at all:
        // it is an arrangement between an operator and an agent, made during a
        // performance, about the node a deck slot happened to be playing. A
        // Set file that carried one would hand that node over wherever it was
        // next loaded. See `Record::Authority`.
        | Record::Authority { .. }
        | Record::Transport { .. }
        | Record::Preview { .. }
        | Record::Transition { .. }
        // **An event, like the `transition` above it, and still dropped — but
        // no longer for the reason it used to be.** The layering *is* a Set
        // file record now: `Record::Merge` says a Set composites, and its
        // `live` field is where a Set records which renderer is the live one.
        // What no *session* record carries is the layering. A `select` is
        // addressed to a deck slot, and nothing in a stream says which slots
        // composite, nor which deck slot the Set at the head of the stream was
        // played in — `session_head` in `karakuri-cli` says as much when it
        // writes one, that "a session stream cannot say what a deck held". So
        // a projection meeting a selection cannot tell whether it is about the
        // Set it is folding, and folding it in would be guessing that the head
        // is the deck slot the record names. When a session record says which
        // deck slot composites, this arm becomes a fold into that slot's
        // `merge`.
        | Record::Select { .. }
        | Record::Mask { .. }
        // **Nothing to fold, and nothing that could be.** A `save` names a Set
        // file that already exists; folding a session down to a Set file is
        // this function's job and it is not the job of the file it names. Two
        // saves in a session are two files, not one with a later answer.
        | Record::Save { .. } => None,
        // **A fourth reason to drop, and it is not a fourth kind of state.** A
        // metadata record describes what an *artifact* declares; a session
        // stream is a performance and no writer here puts one in one. Meeting
        // one means a hand-edited stream, and the answer is to drop it rather
        // than to pass it through: passed through it would reach
        // `Store::write_set`, which refuses it — so a whole save would fail on
        // a line that says nothing about the Set being saved. There is also
        // nothing to fold it onto, which is the same thing said from the other
        // side. See `Record::is_metadata`.
        Record::Meta { .. }
        | Record::ParamDecl { .. }
        | Record::CapacityDecl { .. }
        | Record::Emit { .. } => None,
        Record::Unknown => Some(Key::Passthrough(ordinal)),
    }
}

/// Project a session stream down to the Set file it implies: ticks dropped,
/// state folded to its last value per layer and key.
pub fn project(session: &[Line]) -> Vec<Line> {
    let mut order: Vec<Key> = Vec::new();
    let mut latest: HashMap<Key, Line> = HashMap::new();

    for (i, line) in session.iter().enumerate() {
        let Some(key) = key_for(line.record(), i) else {
            continue;
        };
        if !latest.contains_key(&key) {
            order.push(key.clone());
        }
        latest.insert(key, line.clone());
    }

    order
        .into_iter()
        .map(|key| {
            latest
                .remove(&key)
                .expect("key was just inserted for this exact fold")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Value;

    fn line(record: Record) -> Line {
        Line::new(record)
    }

    #[test]
    fn drops_ticks() {
        let session = vec![
            line(Record::Set {
                id: "s".into(),
                v: 1,
            }),
            line(Record::Tick { steps: 1 }),
            line(Record::Tick { steps: 1 }),
        ];
        let set = project(&session);
        assert_eq!(set.len(), 1);
        assert!(set.iter().all(|l| l.record().is_set_state()));
    }

    /// **The mix is state and is dropped anyway**, which is the one drop here
    /// that is not "there was nothing to fold".
    ///
    /// A gain, a residency and a look describe the deck, and a Set file that
    /// carried them would apply them to whatever deck slot it was next loaded
    /// into: a Set opened into deck slot 0 would pull deck slot 2's fader down,
    /// and one whose
    /// residency said `live` would go on air by being opened. The folding
    /// machinery would happily key them and produce a stable projection — it is
    /// *correct* folding into the wrong file — so nothing but this test stands
    /// between the two scopes.
    #[test]
    fn drops_the_mix_because_it_is_the_sessions_state_and_not_a_sets() {
        let session = vec![
            line(Record::Set {
                id: "s".into(),
                v: 1,
            }),
            line(Record::Gain {
                slot: 2,
                value: 0.25,
            }),
            line(Record::Residency {
                slot: 0,
                level: "live".into(),
            }),
            line(Record::Look {
                op: "agx".into(),
                exposure: 2.0,
                white_point: 4.0,
            }),
            line(Record::Tick { steps: 1 }),
            // A second value for the same key, so a projection that folded
            // these would still produce exactly one line each and look right.
            line(Record::Gain {
                slot: 2,
                value: 0.9,
            }),
        ];
        let set = project(&session);
        assert_eq!(
            set.len(),
            1,
            "the mix reached a Set file: {:?}",
            set.iter().map(|l| l.record()).collect::<Vec<_>>()
        );
        assert!(set.iter().all(|l| l.record().is_set_state()));
    }

    /// A measurement and a correction are what a *frame* saw and decided, not
    /// what anything is, so the projection drops them the way it drops ticks —
    /// including the last one, which is the tempting thing to fold down and
    /// call the session's state.
    #[test]
    fn drops_audio_and_tempo_too() {
        let session = vec![
            line(Record::Set {
                id: "s".into(),
                v: 1,
            }),
            line(Record::Audio {
                energy: 0.4,
                onset: 0.0,
                bands: vec![0.1, 0.2],
                confidence: 1.0,
            }),
            line(Record::Tempo {
                bpm: 128.0,
                shift: -0.01,
                confidence: 0.8,
            }),
            line(Record::Tick { steps: 1 }),
            line(Record::Audio {
                energy: 0.9,
                onset: 1.0,
                bands: vec![0.9, 0.8],
                confidence: 1.0,
            }),
            line(Record::Tempo {
                bpm: 128.1,
                shift: 0.0,
                confidence: 0.9,
            }),
        ];
        let set = project(&session);
        assert_eq!(set.len(), 1);
        assert!(set.iter().all(|l| l.record().is_set_state()));
    }

    #[test]
    fn last_write_wins_per_layer_and_key() {
        let session = vec![
            line(Record::Param {
                layer: Layer::L1,
                index: None,
                key: "radius".into(),
                value: Value::Scalar(2.0),
            }),
            line(Record::Tick { steps: 1 }),
            line(Record::Param {
                layer: Layer::L1,
                index: None,
                key: "radius".into(),
                value: Value::Scalar(2.6),
            }),
            line(Record::Tick { steps: 1 }),
        ];
        let set = project(&session);
        assert_eq!(set.len(), 1);
        assert_eq!(
            set[0].record(),
            &Record::Param {
                layer: Layer::L1,
                index: None,
                key: "radius".into(),
                value: Value::Scalar(2.6)
            }
        );
    }

    #[test]
    fn distinct_keys_stay_distinct() {
        let session = vec![
            line(Record::Param {
                layer: Layer::L1,
                index: None,
                key: "radius".into(),
                value: Value::Scalar(2.0),
            }),
            line(Record::Param {
                layer: Layer::L4,
                index: None,
                key: "radius".into(),
                value: Value::Scalar(0.5),
            }),
            line(Record::Param {
                layer: Layer::L1,
                index: None,
                key: "turbulence".into(),
                value: Value::Scalar(0.8),
            }),
        ];
        let set = project(&session);
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn first_occurrence_position_is_kept_on_update() {
        let session = vec![
            line(Record::Capacity {
                layer: Layer::L1,
                index: 0,
                value: 65536,
            }),
            line(Record::Param {
                layer: Layer::L1,
                index: None,
                key: "radius".into(),
                value: Value::Scalar(2.0),
            }),
            line(Record::Capacity {
                layer: Layer::L1,
                index: 0,
                value: 524288,
            }),
        ];
        let set = project(&session);
        assert_eq!(set.len(), 2);
        // Capacity keeps its original (first) position, but the updated value.
        assert_eq!(
            set[0].record(),
            &Record::Capacity {
                layer: Layer::L1,
                index: 0,
                value: 524288
            }
        );
        assert_eq!(
            set[1].record(),
            &Record::Param {
                layer: Layer::L1,
                index: None,
                key: "radius".into(),
                value: Value::Scalar(2.0)
            }
        );
    }

    /// **Two geometries at two capacities are two records**, which is the whole
    /// reason the record gained an address. Keyed by layer alone they folded
    /// onto each other and the projection kept whichever line came last: a
    /// session in which the operator resized the second source saved as a Set
    /// file that resizes the first, and nothing anywhere said so.
    #[test]
    fn two_geometries_at_two_capacities_do_not_fold_together() {
        let session = vec![
            line(Record::Capacity {
                layer: Layer::L1,
                index: 0,
                value: 65536,
            }),
            line(Record::Capacity {
                layer: Layer::L1,
                index: 1,
                value: 4096,
            }),
            line(Record::Tick { steps: 1 }),
            // The same node again, which *is* a correction — so the fold still
            // has something to do and this is not a test that folding stopped.
            line(Record::Capacity {
                layer: Layer::L1,
                index: 0,
                value: 524288,
            }),
        ];
        let set = project(&session);
        assert_eq!(
            set.len(),
            2,
            "one capacity per geometry: {:?}",
            set.iter().map(|l| l.record()).collect::<Vec<_>>()
        );
        assert_eq!(
            set[0].record(),
            &Record::Capacity {
                layer: Layer::L1,
                index: 0,
                value: 524288
            }
        );
        assert_eq!(
            set[1].record(),
            &Record::Capacity {
                layer: Layer::L1,
                index: 1,
                value: 4096
            }
        );
    }

    /// **A salt belongs to a source**, and two sources salted alike is the one
    /// thing salting exists to prevent: it is what makes two identical grids
    /// differ in colour without being arranged to. A fold keyed by layer alone
    /// gave the pair one seed record and so one randomness.
    #[test]
    fn each_source_keeps_its_own_salt() {
        let session = vec![
            line(Record::Seed {
                stream: Layer::L1,
                index: 0,
                value: 19274,
            }),
            line(Record::Seed {
                stream: Layer::L1,
                index: 1,
                value: 5,
            }),
            line(Record::Tick { steps: 1 }),
            line(Record::Seed {
                stream: Layer::L1,
                index: 1,
                value: 6,
            }),
        ];
        let set = project(&session);
        assert_eq!(
            set.len(),
            2,
            "one salt per source: {:?}",
            set.iter().map(|l| l.record()).collect::<Vec<_>>()
        );
        assert_eq!(
            set[0].record(),
            &Record::Seed {
                stream: Layer::L1,
                index: 0,
                value: 19274
            }
        );
        // Re-seeded, folded onto itself, and still the second source's.
        assert_eq!(
            set[1].record(),
            &Record::Seed {
                stream: Layer::L1,
                index: 1,
                value: 6
            }
        );
    }

    /// **A node renamed is one node, not two.**
    ///
    /// The address is what a `slot` record is folded *by*; the name is one of
    /// the things it says *about* the node it addresses, exactly as `proc` is.
    /// So a session that named a source and then renamed it projects to one
    /// line carrying the later name. A fold keyed by the name would leave a Set
    /// file describing two sources that never existed at once — and would move
    /// a name between nodes when two of them were called the same thing at
    /// different times.
    #[test]
    fn renaming_a_node_folds_onto_it_rather_than_forking_it() {
        let proc_hash = Hash::of(b"proc p { kind L1 }");
        let session = vec![
            line(Record::Slot {
                layer: Layer::L1,
                index: 0,
                name: Some("near".into()),
                proc_hash,
            }),
            line(Record::Tick { steps: 1 }),
            line(Record::Slot {
                layer: Layer::L1,
                index: 0,
                name: Some("veil".into()),
                proc_hash,
            }),
            // A different node that was called what the first one used to be
            // called. Two nodes, and no name is shared at any one instant.
            line(Record::Slot {
                layer: Layer::L1,
                index: 1,
                name: Some("near".into()),
                proc_hash,
            }),
        ];
        let set = project(&session);
        assert_eq!(
            set.len(),
            2,
            "a rename is not a second node: {:?}",
            set.iter().map(|l| l.record()).collect::<Vec<_>>()
        );
        assert_eq!(
            set[0].record(),
            &Record::Slot {
                layer: Layer::L1,
                index: 0,
                name: Some("veil".into()),
                proc_hash
            }
        );
        assert_eq!(
            set[1].record(),
            &Record::Slot {
                layer: Layer::L1,
                index: 1,
                name: Some("near".into()),
                proc_hash
            }
        );
    }

    /// **One L5, so one `merge` record**, and a session that composited and
    /// then said so again folds to the later line.
    ///
    /// The key carries no index because there is nothing for one to
    /// distinguish — a Set has exactly one L5, the node its single `Texture`
    /// output comes out of — so this is `capacity`'s fold over an address with
    /// nothing in it.
    #[test]
    fn a_later_merge_folds_onto_the_earlier_one() {
        let session = vec![
            line(Record::Set {
                id: "s".into(),
                v: 1,
            }),
            line(Record::Merge { live: None }),
            line(Record::Tick { steps: 1 }),
            line(Record::Merge { live: Some(1) }),
        ];
        let set = project(&session);
        assert_eq!(
            set.len(),
            2,
            "a Set holds one L5: {:?}",
            set.iter().map(|l| l.record()).collect::<Vec<_>>()
        );
        assert_eq!(set[1].record(), &Record::Merge { live: Some(1) });
    }

    /// **A selection is dropped even where the stream says the Set
    /// composites**, which is the drop this projection has to keep making and
    /// the one whose reason moved.
    ///
    /// The layering is a Set file record now, so the old reason — "a folded
    /// selection would describe a Set that loads back with every renderer
    /// folded again" — is gone. What replaces it is that **no session record
    /// carries the layering**: a `select` is addressed to a *deck slot*, and
    /// nothing in a stream says which slots composite nor which deck slot the
    /// Set at the head of the stream was played in. The `merge` line below is
    /// the head's, so this session says as much as any session can, and it
    /// still does not say that deck slot 1 is the one this Set is in. Folding the
    /// selection would be guessing that.
    ///
    /// So the `merge` passes through as it was written — `live` still absent,
    /// every input live — and the selection does not reach the file.
    #[test]
    fn a_selection_is_dropped_even_where_the_stream_says_the_set_composites() {
        let session = vec![
            line(Record::Set {
                id: "s".into(),
                v: 1,
            }),
            line(Record::Merge { live: None }),
            line(Record::Tick { steps: 1 }),
            line(Record::Select {
                slot: 1,
                renderer: 2,
                start: 64.0,
            }),
            line(Record::Tick { steps: 1 }),
        ];
        let set = project(&session);
        assert_eq!(
            set.len(),
            2,
            "a selection reached a Set file: {:?}",
            set.iter().map(|l| l.record()).collect::<Vec<_>>()
        );
        assert!(set.iter().all(|l| l.record().is_set_state()));
        assert_eq!(
            set[1].record(),
            &Record::Merge { live: None },
            "the selection was folded into the merge, which no session record \
             says is the merge it is about"
        );
    }

    #[test]
    fn unknown_records_pass_through_unfolded() {
        let session = vec![
            line(Record::Unknown),
            line(Record::Unknown),
            line(Record::Set {
                id: "s".into(),
                v: 1,
            }),
        ];
        let set = project(&session);
        // Both unknown lines survive, distinct from each other.
        assert_eq!(
            set.iter()
                .filter(|l| l.record() == &Record::Unknown)
                .count(),
            2
        );
        assert_eq!(set.len(), 3);
    }
}
