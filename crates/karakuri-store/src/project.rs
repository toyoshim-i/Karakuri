//! Set-file projection: folds live session streams into Set state (last-write-wins per address and key).
//! Unknown record types pass through unfolded to preserve third-party or forward-compatible data.

use std::collections::HashMap;

use crate::hash::Hash;
use crate::ndjson::Line;
use crate::record::{InputPort, Layer, NodeAddress, Record};

#[derive(Clone, PartialEq, Eq, Hash)]
enum Key {
    Header,
    Set,
    Slot(NodeAddress),
    Capacity(NodeAddress),
    Param(Option<NodeAddress>, String),
    Bind(Layer, Option<u32>, String),
    Camera(u32),
    /// No index beside it, because a Set holds exactly one L5. Every other key here
    /// carries whatever says *which* node a record is about; a `merge` record
    /// addresses the only node it could address, so the key is the record type and
    /// a second `merge` in a session is one L5 with a later answer.
    Merge,
    Seed(Layer, u32),
    /// A node and the *input slot* of it being bound — the two halves of what an
    /// edge is *about*, where the node it is bound *to* is what the edge says.
    Edge(String, InputPort),
    Src(Hash, u32),
    /// An unfoldable line (currently only `Record::Unknown`), identified by its
    /// position in the input so it never coalesces with another.
    Passthrough(usize),
}

/// Returns the projection fold key for a record, or `None` if the record does
/// not belong to persistent Set state (e.g. transient ticks, deck-level controls,
/// metadata declarations, or unresolved relative part paths).
fn key_for(record: &Record, ordinal: usize) -> Option<Key> {
    match record {
        Record::Header { .. } => Some(Key::Header),
        Record::Set { .. } => Some(Key::Set),
        // Keyed by node address (layer, index) so renames fold onto the existing node.
        Record::Slot { at, .. } => Some(Key::Slot(*at)),
        // Keyed by node address so distinct nodes on the same layer remain distinct.
        Record::Capacity { at, .. } => Some(Key::Capacity(*at)),
        // Keyed by node address (or wildcard None) and parameter key.
        Record::Param { at, key, .. } => Some(Key::Param(*at, key.clone())),
        Record::Bind {
            layer, index, key, ..
        } => Some(Key::Bind(*layer, *index, key.clone())),
        Record::Camera { index, .. } => Some(Key::Camera(*index)),
        Record::Merge { .. } => Some(Key::Merge),
        Record::Seed { stream, index, .. } => Some(Key::Seed(*stream, *index)),
        // Keyed by destination node and input port so rebindings overwrite.
        Record::Edge { node, slot, .. } => Some(Key::Edge(node.clone(), slot.clone())),
        Record::Src { hash, line, .. } => Some(Key::Src(*hash, *line)),
        // Non-Set-state or transient session records are excluded from Set projections.
        Record::Tick { .. }
        | Record::Audio { .. }
        | Record::Tempo { .. }
        | Record::Gain { .. }
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
        | Record::Save { .. }
        | Record::Meta { .. }
        | Record::ParamDecl { .. }
        | Record::CapacityDecl { .. }
        | Record::Emit { .. }
        | Record::Part { .. } => None,
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
    use crate::record::{DeckSlot, Value};

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

    /// Verifies that session mix state (Gain, Residency, Look) is excluded from Set files.
    #[test]
    fn drops_the_mix_because_it_is_the_sessions_state_and_not_a_sets() {
        let session = vec![
            line(Record::Set {
                id: "s".into(),
                v: 1,
            }),
            line(Record::Gain {
                slot: DeckSlot(2),
                value: 0.25,
            }),
            line(Record::Residency {
                slot: DeckSlot(0),
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
                slot: DeckSlot(2),
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

    /// Verifies that transient audio analysis and tempo records are excluded.
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
                at: None,
                key: "radius".into(),
                value: Value::Scalar(2.0),
            }),
            line(Record::Tick { steps: 1 }),
            line(Record::Param {
                at: None,
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
                at: None,
                key: "radius".into(),
                value: Value::Scalar(2.6)
            }
        );
    }

    /// Verifies that deck-slot ride records are dropped rather than folded into Set parameters.
    #[test]
    fn a_ride_is_dropped_where_a_param_at_the_same_address_is_folded() {
        let session = vec![
            line(Record::Param {
                at: Some(NodeAddress {
                    layer: Layer::L1,
                    index: 0,
                }),
                key: "radius".into(),
                value: Value::Scalar(2.0),
            }),
            line(Record::Tick { steps: 1 }),
            line(Record::Ride {
                slot: DeckSlot(0),
                at: Some(crate::record::NodeAddress {
                    layer: Layer::L1,
                    index: 0,
                }),
                key: "radius".into(),
                value: Value::Scalar(9.0),
            }),
            line(Record::Ride {
                slot: DeckSlot(3),
                at: Some(crate::record::NodeAddress {
                    layer: Layer::L1,
                    index: 0,
                }),
                key: "radius".into(),
                value: Value::Scalar(7.0),
            }),
            line(Record::Tick { steps: 1 }),
        ];
        let set = project(&session);
        assert_eq!(
            set.len(),
            1,
            "a ride reached the Set file: {:?}",
            set.iter().map(|l| l.record()).collect::<Vec<_>>()
        );
        assert_eq!(
            set[0].record(),
            &Record::Param {
                at: Some(NodeAddress {
                    layer: Layer::L1,
                    index: 0
                }),
                key: "radius".into(),
                value: Value::Scalar(2.0)
            },
            "the param the Set was loaded with is what a fold of this session says"
        );
    }

    /// Verifies that parameters with differing keys or addresses remain distinct.
    #[test]
    fn distinct_keys_stay_distinct() {
        let session = vec![
            line(Record::Param {
                at: None,
                key: "radius".into(),
                value: Value::Scalar(2.0),
            }),
            line(Record::Param {
                at: Some(NodeAddress {
                    layer: Layer::L1,
                    index: 0,
                }),
                key: "radius".into(),
                value: Value::Scalar(0.5),
            }),
            line(Record::Param {
                at: None,
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
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 0,
                },
                value: 65536,
            }),
            line(Record::Param {
                at: None,
                key: "radius".into(),
                value: Value::Scalar(2.0),
            }),
            line(Record::Capacity {
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 0,
                },
                value: 524288,
            }),
        ];
        let set = project(&session);
        assert_eq!(set.len(), 2);
        // Capacity keeps its original (first) position, but the updated value.
        assert_eq!(
            set[0].record(),
            &Record::Capacity {
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 0
                },
                value: 524288
            }
        );
        assert_eq!(
            set[1].record(),
            &Record::Param {
                at: None,
                key: "radius".into(),
                value: Value::Scalar(2.0)
            }
        );
    }

    /// Verifies that distinct node capacities on the same layer do not overwrite each other.
    #[test]
    fn two_geometries_at_two_capacities_do_not_fold_together() {
        let session = vec![
            line(Record::Capacity {
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 0,
                },
                value: 65536,
            }),
            line(Record::Capacity {
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 1,
                },
                value: 4096,
            }),
            line(Record::Tick { steps: 1 }),
            // The same node again, which *is* a correction — so the fold still
            // has something to do and this is not a test that folding stopped.
            line(Record::Capacity {
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 0,
                },
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
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 0
                },
                value: 524288
            }
        );
        assert_eq!(
            set[1].record(),
            &Record::Capacity {
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 1
                },
                value: 4096
            }
        );
    }

    /// Verifies that seed records for distinct sources on the same layer remain distinct.
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

    /// Verifies that renaming a node updates its existing record rather than creating a second node.
    #[test]
    fn renaming_a_node_folds_onto_it_rather_than_forking_it() {
        let proc_hash = Hash::of(b"proc p { kind L1 }");
        let session = vec![
            line(Record::Slot {
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 0,
                },
                name: Some("near".into()),
                proc_hash,
            }),
            line(Record::Tick { steps: 1 }),
            line(Record::Slot {
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 0,
                },
                name: Some("veil".into()),
                proc_hash,
            }),
            // A different node that was called what the first one used to be
            // called. Two nodes, and no name is shared at any one instant.
            line(Record::Slot {
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 1,
                },
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
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 0
                },
                name: Some("veil".into()),
                proc_hash
            }
        );
        assert_eq!(
            set[1].record(),
            &Record::Slot {
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 1
                },
                name: Some("near".into()),
                proc_hash
            }
        );
    }

    /// Verifies that subsequent merge records overwrite earlier ones.
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

    /// Verifies that deck selection events do not modify Set merge records.
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
                slot: DeckSlot(1),
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
