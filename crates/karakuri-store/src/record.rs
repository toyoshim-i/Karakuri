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

/// The noise generator a `bind` declares for itself.
///
/// Every field has a default, and they are the defaults of the generator in
/// `karakuri-signal`: a `bind` that names `noise` and says nothing else gets
/// one cycle per beat of perlin on stream 0. A generator omitted field by
/// field is a generator that was under-specified, not one that was refused —
/// this is the one record whose parameters an LLM has to invent numbers for,
/// and every number here has a defensible one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BindNoise {
    /// `white`, `value`, `perlin`, or `fbm`. A `String` for the same reason
    /// `curve` is one: an unrecognised value is the engine's to diagnose
    /// against what it actually supports, not the decoder's to reject before
    /// anything can say what the alternatives were.
    #[serde(default = "default_noise_kind")]
    pub kind: String,
    /// Cycles per beat, so period is tempo-relative.
    #[serde(default = "default_noise_rate")]
    pub rate: f32,
    /// Decorrelates one binding from another. Two bindings sharing a stream
    /// move together.
    #[serde(default)]
    pub stream: u64,
    /// `fbm` only, ignored by the other three kinds. Not in v0.2 of the spec,
    /// which listed `fbm` as a `kind` while giving it nowhere to say how many
    /// octaves — the spec is corrected rather than the field being dropped,
    /// because an octave count baked into the engine is exactly the "fixed
    /// property nobody can reach" that Spawn timing rejects Poisson for.
    #[serde(default = "default_noise_octaves")]
    pub octaves: u32,
}

fn default_noise_kind() -> String {
    "perlin".to_string()
}

fn default_noise_rate() -> f32 {
    1.0
}

fn default_noise_octaves() -> u32 {
    4
}

impl Default for BindNoise {
    fn default() -> BindNoise {
        BindNoise {
            kind: default_noise_kind(),
            rate: default_noise_rate(),
            stream: 0,
            octaves: default_noise_octaves(),
        }
    }
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
    /// Attaches a signal to one `param` of one layer. The value written every
    /// frame is the signal put through `curve`, mapped onto `range`, and then
    /// blended against the param's own value by the sample's confidence — see
    /// "Set file format" in `docs/ir-spec.md`.
    Bind {
        layer: Layer,
        key: String,
        signal: String,
        curve: String,
        range: [f32; 2],
        /// Only a `signal` of `"noise"` reads this, because a noise generator
        /// is the one signal with parameters of its own. Absent means the
        /// default generator rather than no generator: there is nothing else
        /// for the name to mean.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        noise: Option<BindNoise>,
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
    fn a_bind_without_a_noise_object_round_trips_and_stays_without_one() {
        // The field is absent rather than `null` on the way out: a Set file is
        // read by humans and generated by LLMs, and a `"noise":null` on every
        // ordinary binding teaches both that it is a thing to fill in.
        let rec = round_trip(
            r#"{"t":"bind","layer":"L1","key":"turbulence","signal":"energy","curve":"pow2","range":[0.1,2.4]}"#,
        );
        assert_eq!(
            rec,
            Record::Bind {
                layer: Layer::L1,
                key: "turbulence".into(),
                signal: "energy".into(),
                curve: "pow2".into(),
                range: [0.1, 2.4],
                noise: None,
            }
        );
        assert!(!serde_json::to_string(&rec).unwrap().contains("noise"));
    }

    #[test]
    fn a_noise_bind_round_trips_with_its_generator() {
        // The example out of `docs/ir-spec.md`'s "Binding noise", verbatim.
        let rec = round_trip(
            r#"{"t":"bind","layer":"L1","key":"spawn_rate","signal":"noise",
                "noise":{"kind":"perlin","rate":0.5,"stream":3},"curve":"lin","range":[4000,16000]}"#,
        );
        let Record::Bind { noise: Some(n), .. } = rec else {
            panic!("expected a bind carrying a noise generator");
        };
        assert_eq!(n.kind, "perlin");
        assert_eq!(n.rate, 0.5);
        assert_eq!(n.stream, 3);
        // Absent in the spec's own example, so it has to have a default or the
        // example does not decode.
        assert_eq!(n.octaves, 4);
    }

    #[test]
    fn an_empty_noise_object_is_the_default_generator() {
        // Field by field: a generator an LLM under-specified is one that runs,
        // not one that is refused. `{}` is the extreme case of that.
        let rec: Record = serde_json::from_str(
            r#"{"t":"bind","layer":"L1","key":"k","signal":"noise","curve":"lin","range":[0,1],"noise":{}}"#,
        )
        .expect("parse");
        let Record::Bind { noise: Some(n), .. } = rec else {
            panic!("expected a bind carrying a noise generator");
        };
        assert_eq!(n, BindNoise::default());
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
