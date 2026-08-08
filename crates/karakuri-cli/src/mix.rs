//! The mix, through the record stream on the way.
//!
//! The faders, the blend modes, residency and the output look are the state an
//! operator moves during a performance and the only state the engine had no
//! record vocabulary for at all — `README.md`'s Invariants named the gap. A
//! session that replayed everything else would replay the material and not the
//! *performance*: the same Sets, on the same beat, all at whatever gain they
//! happened to start at, with nothing ever going on or off air.
//!
//! ```text
//!   a key press ─→ Record::Gain      ─┐
//!                  Record::Opacity    │
//!                  Record::Blend      ┼→ Change ─→ Deck / Present
//!                  Record::Residency  │
//!                  Record::Look      ─┘
//! ```
//!
//! **Built and read back, never applied directly**, which is the same
//! arrangement `audio.rs` has and is there for the same reason: the path the
//! engine is driven through is the record's rather than one that happens to
//! agree with it. A decode that only a test exercises is a decode that is
//! correct until the day it matters.
//!
//! Not on the frame path — a fader moves when a hand moves it — so the `String`
//! a `residency` or a `look` record carries is a key press's allocation and not
//! a frame's. `audio.rs`'s record is reused in place precisely because that one
//! *is* on the frame path; this one does not need to be, and pretending it did
//! would be complexity bought with nothing.
//!
//! ## Opacity, which used to be deliberately not here
//!
//! `Deck::set_opacity` existed with no key, no flag and no record, and this
//! module said so: a record type for a control the operator cannot move is one
//! more record nobody writes, which is the condition it exists to end rather
//! than extend. **It got a record when it got a control**, and it got a control
//! when [`karakuri_engine::deck::Blend`] made it mean something a gain does not
//! — the fader across the blend rather than the level the material arrives at.
//! Under `add` the two multiply together and a stream carrying either would
//! replay the same; under `over` one dims a layer and the other stops it
//! hiding what is beneath.

use karakuri_engine::deck::{Blend, Residency};
use karakuri_engine::transport::{Sync, Transport};
use karakuri_store::record::Record;

use crate::{op_wire_name, op_wire_names, Look};

/// What one mix record says, decoded into what the engine takes.
///
/// The engine's own types, not the record's: a `Residency` rather than the
/// string it was spelled with, a [`Look`] rather than three loose fields. That
/// is where the decode ends and it is the whole of what the caller applies.
#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub enum Change {
    Gain { slot: usize, value: f32 },
    /// The fader. Separate from `Gain` because the blend mode makes them
    /// separate — see [`Blend`].
    Opacity { slot: usize, value: f32 },
    Blend { slot: usize, mode: Blend },
    Residency { slot: usize, level: Residency },
    Look(Look),
    /// What a slot's clock does with the session's. Carried as a value rather
    /// than applied as a mode change, because the record says all three and a
    /// replay must not recompute one of them from the machine it is on.
    Transport {
        slot: usize,
        sync: Sync,
        anchor_bpm: f32,
        offset_beats: f64,
    },
}

/// A slot's gain, as the record that carries it.
pub fn gain_record(slot: usize, value: f32) -> Record {
    Record::Gain {
        slot: slot as u8,
        value,
    }
}

/// A slot's opacity — the fader — as the record that carries it.
pub fn opacity_record(slot: usize, value: f32) -> Record {
    Record::Opacity {
        slot: slot as u8,
        value,
    }
}

/// A slot's blend mode, as the record that carries it.
pub fn blend_record(slot: usize, mode: Blend) -> Record {
    Record::Blend {
        slot: slot as u8,
        mode: mode.name().to_string(),
    }
}

/// A slot's **requested** residency, as the record that carries it. See
/// [`Record::Residency`] for why the effective one is not recordable.
pub fn residency_record(slot: usize, level: Residency) -> Record {
    Record::Residency {
        slot: slot as u8,
        level: residency_wire_name(level).to_string(),
    }
}

/// The output look, as the record that carries it.
pub fn look_record(look: &Look) -> Record {
    Record::Look {
        op: op_wire_name(look.op).to_string(),
        exposure: look.exposure,
        white_point: look.white_point,
    }
}

/// A slot's transport, as the record that carries it.
pub fn transport_record(slot: usize, transport: &Transport) -> Record {
    Record::Transport {
        slot: slot as u8,
        sync: transport.sync().name().to_string(),
        anchor_bpm: transport.anchor_bpm(),
        offset_beats: transport.offset_beats(),
    }
}

/// Every residency level there is, with its length in its type. `Residency` is
/// the engine's and has no iterator, so this is the list — and
/// [`residency_wire_name`] below is the exhaustive match that stops a level
/// from reaching the wire without a name.
pub const LEVELS: [Residency; 3] = [
    Residency::Live,
    Residency::Priming,
    Residency::Allocated,
];

/// The wire spelling of a residency level. Lower case and stable; the status
/// line's `LIVE`/`prim`/`park` are a different vocabulary for a different
/// reader and are deliberately not this one.
///
/// A match rather than a table lookup, so a level added to the engine does not
/// compile until it has a spelling. [`parse_residency`] is derived from this
/// one over [`LEVELS`], so the two directions cannot disagree — the remaining
/// hand-written thing is `LEVELS` itself, and a level missing from it is a
/// record that fails to decode with a message naming what was available,
/// rather than one that decodes as the wrong level.
pub fn residency_wire_name(level: Residency) -> &'static str {
    match level {
        Residency::Live => "live",
        Residency::Priming => "priming",
        Residency::Allocated => "allocated",
    }
}

fn parse_residency(name: &str) -> Option<Residency> {
    LEVELS
        .iter()
        .copied()
        .find(|level| residency_wire_name(*level) == name)
}

/// Every wire spelling, for an error message that says what was available.
fn residency_wire_names() -> String {
    LEVELS
        .iter()
        .map(|level| residency_wire_name(*level))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One mix record as the change it asks for.
///
/// Three answers, and they are three different things:
///
/// - `Ok(None)` — **not a mix record.** A `tick` or an `audio` is not this
///   module's to act on and not an error either.
/// - `Err(_)` — **a mix record this build cannot obey.** An unknown residency
///   level, an unknown tone map operator, a slot the deck does not have. The
///   decoder does not reject these; it reports them, because what a name is
///   allowed to be is the engine's business and only the engine can say what
///   the alternatives were. A stream from a newer build reaches here, not the
///   parser.
/// - `Ok(Some(_))` — what to do.
///
/// `slot_count` is the deck's, so a record naming a slot that does not exist is
/// caught here rather than panicking in an index four frames later.
pub fn change(record: &Record, slot_count: usize) -> Result<Option<Change>, String> {
    let in_range = |slot: u8| -> Result<usize, String> {
        let slot = usize::from(slot);
        if slot < slot_count {
            Ok(slot)
        } else {
            Err(format!(
                "slot {slot}: this deck holds slots 0-{}",
                slot_count.saturating_sub(1)
            ))
        }
    };
    match record {
        Record::Gain { slot, value } => Ok(Some(Change::Gain {
            slot: in_range(*slot)?,
            value: *value,
        })),
        Record::Opacity { slot, value } => Ok(Some(Change::Opacity {
            slot: in_range(*slot)?,
            value: *value,
        })),
        Record::Blend { slot, mode } => {
            let slot = in_range(*slot)?;
            let mode = Blend::from_name(mode).ok_or_else(|| {
                format!(
                    "blend `{mode}` — expected {}",
                    Blend::ALL
                        .iter()
                        .map(|b| b.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            Ok(Some(Change::Blend { slot, mode }))
        }
        Record::Residency { slot, level } => {
            let slot = in_range(*slot)?;
            let level = parse_residency(level).ok_or_else(|| {
                format!("residency `{level}` — expected {}", residency_wire_names())
            })?;
            Ok(Some(Change::Residency { slot, level }))
        }
        Record::Transport {
            slot,
            sync,
            anchor_bpm,
            offset_beats,
        } => {
            let slot = in_range(*slot)?;
            let sync = Sync::from_name(sync).ok_or_else(|| {
                format!(
                    "sync `{sync}` — expected {}",
                    Sync::ALL
                        .iter()
                        .map(|s| s.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            Ok(Some(Change::Transport {
                slot,
                sync,
                anchor_bpm: *anchor_bpm,
                offset_beats: *offset_beats,
            }))
        }
        Record::Look {
            op,
            exposure,
            white_point,
        } => {
            let op = crate::parse_op(op)
                .ok_or_else(|| format!("tonemap `{op}` — expected {}", op_wire_names()))?;
            Ok(Some(Change::Look(Look {
                op,
                exposure: *exposure,
                white_point: *white_point,
            })))
        }
        _ => Ok(None),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use karakuri_engine::TonemapOp;

    /// The round trip, which is the whole claim: what the live path builds is
    /// what a replay would decode, for every mix record there is.
    #[test]
    fn every_mix_record_survives_the_json_between() {
        let cases = [
            (gain_record(2, 0.75), Change::Gain { slot: 2, value: 0.75 }),
            (
                opacity_record(0, 0.25),
                Change::Opacity {
                    slot: 0,
                    value: 0.25,
                },
            ),
            (
                blend_record(3, Blend::Over),
                Change::Blend {
                    slot: 3,
                    mode: Blend::Over,
                },
            ),
            (
                residency_record(1, Residency::Priming),
                Change::Residency {
                    slot: 1,
                    level: Residency::Priming,
                },
            ),
            (
                transport_record(3, &Transport::engaged(Sync::Beat, 126.0)),
                Change::Transport {
                    slot: 3,
                    sync: Sync::Beat,
                    anchor_bpm: 126.0,
                    offset_beats: 0.0,
                },
            ),
            (
                look_record(&Look {
                    op: TonemapOp::AgX,
                    exposure: 1.5,
                    white_point: 4.0,
                }),
                Change::Look(Look {
                    op: TonemapOp::AgX,
                    exposure: 1.5,
                    white_point: 4.0,
                }),
            ),
        ];
        for (record, expected) in cases {
            let line = serde_json::to_string(&record).expect("serialise");
            let decoded: Record = serde_json::from_str(&line).expect("parse");
            assert_eq!(
                change(&decoded, 4).expect("a record this build built"),
                Some(expected),
                "through {line}"
            );
        }
    }

    /// **Every residency level round-trips, not just the one a test remembered
    /// to name.** A level added to the engine and not to the wire vocabulary
    /// would otherwise be a slot silently refusing to change state.
    #[test]
    fn every_residency_level_has_a_wire_name_that_decodes_back() {
        for level in LEVELS {
            let record = residency_record(0, level);
            assert_eq!(
                change(&record, 1).expect("built here"),
                Some(Change::Residency { slot: 0, level }),
                "{level:?} did not survive its own wire name"
            );
        }
    }

    /// **Every blend mode round-trips**, so a mode added to the engine and not
    /// to the wire vocabulary is a layer silently composited the wrong way —
    /// which under `over` is a layer that was supposed to hide and does not.
    #[test]
    fn every_blend_mode_has_a_wire_name_that_decodes_back() {
        for mode in Blend::ALL {
            let record = blend_record(2, mode);
            assert_eq!(
                change(&record, 4).expect("built here"),
                Some(Change::Blend { slot: 2, mode }),
                "{} did not survive its own wire name",
                mode.name()
            );
        }
    }

    /// **Every sync mode round-trips**, so a mode added to the engine and not
    /// to the wire vocabulary is a slot silently left free rather than put
    /// where the record said.
    #[test]
    fn every_sync_mode_has_a_wire_name_that_decodes_back() {
        for sync in Sync::ALL {
            let mut transport = Transport::engaged(sync, 100.0);
            transport.scrub(-0.75);
            let record = transport_record(1, &transport);
            assert_eq!(
                change(&record, 4).expect("built here"),
                Some(Change::Transport {
                    slot: 1,
                    sync,
                    anchor_bpm: 100.0,
                    // Carried under every mode, including the two that do
                    // nothing with it — a slot moved back onto the grid returns
                    // to where the operator left it.
                    offset_beats: -0.75,
                }),
                "{} did not survive its own wire name",
                sync.name()
            );
        }
    }

    /// Every tone map operator likewise, through `look`. `TONEMAPS` is one
    /// table now, so this fails if an operator is added to the enum and not to
    /// it.
    #[test]
    fn every_tonemap_operator_has_a_wire_name_that_decodes_back() {
        for op in crate::TONEMAPS {
            let look = Look {
                op,
                exposure: 1.0,
                white_point: 4.0,
            };
            let Some(Change::Look(decoded)) =
                change(&look_record(&look), 1).expect("built here")
            else {
                panic!("a look record did not decode as a look");
            };
            assert_eq!(decoded.op, op, "{} did not survive its wire name", op_wire_name(op));
        }
    }

    /// A record this build cannot obey is **reported, not dropped**. Silently
    /// ignoring it would leave a session replaying at the wrong gain with
    /// nothing said, which is worse than refusing the line.
    #[test]
    fn a_record_this_build_cannot_obey_says_so_rather_than_vanishing() {
        let unknown_level = Record::Residency {
            slot: 0,
            level: "cooling".to_string(),
        };
        let message = change(&unknown_level, 4).expect_err("`cooling` is not a level here");
        assert!(message.contains("cooling"), "{message}");
        assert!(message.contains("live"), "{message}");

        let unknown_op = Record::Look {
            op: "filmic".to_string(),
            exposure: 1.0,
            white_point: 4.0,
        };
        let message = change(&unknown_op, 4).expect_err("`filmic` is not an operator here");
        assert!(message.contains("filmic"), "{message}");
        assert!(message.contains("aces"), "{message}");

        let unknown_mode = Record::Blend {
            slot: 0,
            mode: "screen".to_string(),
        };
        let message = change(&unknown_mode, 4).expect_err("`screen` is not a mode here");
        assert!(message.contains("screen"), "{message}");
        assert!(message.contains("over"), "{message}");
    }

    /// **A slot the deck does not have is caught in the decode**, where there
    /// is something to say about it, rather than four frames later in an index.
    #[test]
    fn a_slot_past_the_deck_is_refused_with_the_range_it_missed() {
        let message = change(&gain_record(4, 1.0), 4).expect_err("slot 4 of a deck of 4");
        assert!(message.contains("slots 0-3"), "{message}");
        // And the boundary either side of it, which is where the off-by-one
        // this shares with the digit keys would live.
        assert!(change(&gain_record(3, 1.0), 4).is_ok());
        assert!(change(&gain_record(0, 1.0), 1).is_ok());
        assert!(change(&gain_record(1, 1.0), 1).is_err());
    }

    /// A record that is not the mix's is not an error. `audio.rs` decodes
    /// those, and both decoders see every record a session carries.
    #[test]
    fn a_record_that_is_not_the_mixs_is_left_alone() {
        assert_eq!(change(&Record::Tick { steps: 1 }, 4), Ok(None));
    }
}
