//! ndjson records.
//!
//! One record per line, and concatenation is composition. The same record types
//! serve two files:
//!
//! - a **Set file** (`.set.ndjson`) is a state projection — what is loaded and
//!   what every value currently is. It carries no time, so it never contains
//!   [`Record::Tick`].
//! - a **session stream** is a timeline — a Set file followed by ticks and the
//!   edits between them. Every edit lands at an exact frame position because it
//!   sits between two known ticks.
//!
//! A Set file is the session stream with the ticks dropped and the state folded
//! down. See the Set file and session stream sections of `docs/ir-spec.md`.

use serde::{Deserialize, Serialize};

use crate::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Layer {
    L1,
    L2,
    L3,
    L4,
}

/// A parameter value. Ranges are declared in the `.kir`; this is just the value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Scalar(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Record {
    Set {
        id: String,
        v: u32,
    },
    Slot {
        layer: Layer,
        #[serde(rename = "proc")]
        proc_hash: Hash,
    },
    /// Overrides the `.kir` default. Outside the range the artifact declares,
    /// the Set is rejected at build time.
    Capacity {
        layer: Layer,
        value: u32,
    },
    Param {
        layer: Layer,
        key: String,
        value: Value,
    },
    Bind {
        layer: Layer,
        key: String,
        signal: String,
        curve: String,
        range: [f32; 2],
    },
    Camera {
        kind: String,
        radius: f32,
        speed: f32,
    },
    /// Salts the hash builtins for a layer, so re-seeding changes randomness
    /// without touching anything structural.
    Seed {
        stream: Layer,
        value: u64,
    },
    /// Inlined `.kir` source, for bundling an artifact with the Set that uses it.
    Src {
        hash: Hash,
        line: u32,
        s: String,
    },
    /// How far this frame advances. Emitted from real time when live, read back
    /// verbatim on replay — which is what keeps substepping deterministic.
    /// Always 1 in v0.2, capped at [`MAX_STEPS`].
    Tick {
        steps: u8,
    },
    /// Forward compatibility: an unrecognised `t` is ignored, not an error.
    #[serde(other)]
    Unknown,
}

/// Beyond this the simulation is allowed to fall behind rather than catch up.
/// Unbounded catch-up turns a load spike into a death spiral.
pub const MAX_STEPS: u8 = 4;

impl Record {
    /// Whether this record belongs in a Set file. Ticks do not: a Set file
    /// carries no time.
    pub fn is_state(&self) -> bool {
        !matches!(self, Record::Tick { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(line: &str) -> Record {
        let rec: Record = serde_json::from_str(line).expect("parse");
        let back = serde_json::to_string(&rec).expect("serialise");
        let again: Record = serde_json::from_str(&back).expect("reparse");
        assert_eq!(rec, again);
        rec
    }

    #[test]
    fn tick_round_trips() {
        assert_eq!(round_trip(r#"{"t":"tick","steps":1}"#), Record::Tick { steps: 1 });
    }

    #[test]
    fn capacity_override_round_trips() {
        assert_eq!(
            round_trip(r#"{"t":"capacity","layer":"L1","value":524288}"#),
            Record::Capacity {
                layer: Layer::L1,
                value: 524288
            }
        );
    }

    #[test]
    fn unknown_records_are_ignored_not_rejected() {
        // Forward compatibility: a newer engine's record must not break an
        // older reader.
        assert_eq!(
            serde_json::from_str::<Record>(r#"{"t":"phrase","at":4.0}"#).unwrap(),
            Record::Unknown
        );
    }

    #[test]
    fn ticks_are_not_state() {
        assert!(!Record::Tick { steps: 1 }.is_state());
        assert!(Record::Set {
            id: "drift_01".into(),
            v: 1
        }
        .is_state());
    }
}
