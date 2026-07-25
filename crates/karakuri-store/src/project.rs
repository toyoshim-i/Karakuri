//! The Set-file projection: "a Set file is the session stream with ticks
//! dropped and the state folded down" (`docs/ir-spec.md`, Session stream
//! format).
//!
//! Folding is last-write-wins per layer and key, where "key" depends on the
//! record type — a [`Record::Param`] is keyed by `(layer, key)`, a
//! [`Record::Capacity`] by `layer` alone, [`Record::Camera`] is a
//! singleton, and so on. The result keeps each key at the position of its
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
    Slot(Layer),
    Capacity(Layer),
    Param(Layer, String),
    Bind(Layer, String),
    Camera,
    Seed(Layer),
    Src(Hash, u32),
    /// An unfoldable line (currently only `Record::Unknown`), identified by
    /// its position in the input so it never coalesces with another.
    Passthrough(usize),
}

/// The fold key for a record, or `None` if the record carries no state at
/// all and should simply be dropped — that is `Record::Tick`, which is the
/// one thing the spec guarantees never appears in a Set file.
fn key_for(record: &Record, ordinal: usize) -> Option<Key> {
    match record {
        Record::Set { .. } => Some(Key::Set),
        Record::Slot { layer, .. } => Some(Key::Slot(*layer)),
        Record::Capacity { layer, .. } => Some(Key::Capacity(*layer)),
        Record::Param { layer, key, .. } => Some(Key::Param(*layer, key.clone())),
        Record::Bind { layer, key, .. } => Some(Key::Bind(*layer, key.clone())),
        Record::Camera { .. } => Some(Key::Camera),
        Record::Seed { stream, .. } => Some(Key::Seed(*stream)),
        Record::Src { hash, line, .. } => Some(Key::Src(*hash, *line)),
        Record::Tick { .. } => None,
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
        .map(|key| latest.remove(&key).expect("key was just inserted for this exact fold"))
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
            line(Record::Set { id: "s".into(), v: 1 }),
            line(Record::Tick { steps: 1 }),
            line(Record::Tick { steps: 1 }),
        ];
        let set = project(&session);
        assert_eq!(set.len(), 1);
        assert!(set.iter().all(|l| l.record().is_state()));
    }

    #[test]
    fn last_write_wins_per_layer_and_key() {
        let session = vec![
            line(Record::Param {
                layer: Layer::L1,
                key: "radius".into(),
                value: Value::Scalar(2.0),
            }),
            line(Record::Tick { steps: 1 }),
            line(Record::Param {
                layer: Layer::L1,
                key: "radius".into(),
                value: Value::Scalar(2.6),
            }),
            line(Record::Tick { steps: 1 }),
        ];
        let set = project(&session);
        assert_eq!(set.len(), 1);
        assert_eq!(
            set[0].record(),
            &Record::Param { layer: Layer::L1, key: "radius".into(), value: Value::Scalar(2.6) }
        );
    }

    #[test]
    fn distinct_keys_stay_distinct() {
        let session = vec![
            line(Record::Param { layer: Layer::L1, key: "radius".into(), value: Value::Scalar(2.0) }),
            line(Record::Param { layer: Layer::L4, key: "radius".into(), value: Value::Scalar(0.5) }),
            line(Record::Param { layer: Layer::L1, key: "turbulence".into(), value: Value::Scalar(0.8) }),
        ];
        let set = project(&session);
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn first_occurrence_position_is_kept_on_update() {
        let session = vec![
            line(Record::Capacity { layer: Layer::L1, value: 65536 }),
            line(Record::Param { layer: Layer::L1, key: "radius".into(), value: Value::Scalar(2.0) }),
            line(Record::Capacity { layer: Layer::L1, value: 524288 }),
        ];
        let set = project(&session);
        assert_eq!(set.len(), 2);
        // Capacity keeps its original (first) position, but the updated value.
        assert_eq!(set[0].record(), &Record::Capacity { layer: Layer::L1, value: 524288 });
        assert_eq!(
            set[1].record(),
            &Record::Param { layer: Layer::L1, key: "radius".into(), value: Value::Scalar(2.0) }
        );
    }

    #[test]
    fn unknown_records_pass_through_unfolded() {
        let session = vec![
            line(Record::Unknown),
            line(Record::Unknown),
            line(Record::Set { id: "s".into(), v: 1 }),
        ];
        let set = project(&session);
        // Both unknown lines survive, distinct from each other.
        assert_eq!(set.iter().filter(|l| l.record() == &Record::Unknown).count(), 2);
        assert_eq!(set.len(), 3);
    }
}
