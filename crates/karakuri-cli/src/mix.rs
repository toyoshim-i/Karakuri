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
//!                  Record::Blend      │
//!                  Record::Preview    ┼→ Change ─→ Deck / Present
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

use karakuri_engine::binding::Curve;
use karakuri_engine::deck::{Blend, Mask, MaskKind, Residency};
use karakuri_engine::transition::Control;
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
    Mask { slot: usize, mask: Mask },
    /// Which slot the output is showing, or `None` for the mix. Not a mix
    /// control; see [`Record::Preview`] for why it is in the stream anyway and
    /// for when it will stop being.
    Preview { slot: Option<usize> },
    /// A scheduled move. Carried as its parts rather than as a
    /// `karakuri_engine::Transition`, because building one needs the value the
    /// control is at *now* and that is the applier's to read, not the decoder's.
    Transition {
        slot: usize,
        control: Control,
        to: f32,
        start: f64,
        beats: f64,
        curve: Curve,
    },
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

/// A slot's mask, as the record that carries it.
pub fn mask_record(slot: usize, mask: Mask) -> Record {
    Record::Mask {
        slot: slot as u8,
        kind: mask.kind().name().to_string(),
        angle: mask.angle(),
        position: mask.position(),
        softness: mask.softness(),
    }
}

/// A scheduled move, as the record that carries it.
pub fn transition_record(
    slot: usize,
    control: Control,
    to: f32,
    start: f64,
    beats: f64,
    curve: Curve,
) -> Record {
    Record::Transition {
        slot: slot as u8,
        control: control.name().to_string(),
        to,
        start,
        beats,
        curve: curve.name().to_string(),
    }
}

/// Which slot is being auditioned, as the record that carries it.
pub fn preview_record(slot: Option<usize>) -> Record {
    Record::Preview {
        slot: slot.map(|s| s as u8),
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
        Record::Mask {
            slot,
            kind,
            angle,
            position,
            softness,
        } => {
            let slot = in_range(*slot)?;
            let kind = MaskKind::from_name(kind).ok_or_else(|| {
                format!(
                    "mask `{kind}` — expected {}",
                    MaskKind::ALL
                        .iter()
                        .map(|k| k.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            Ok(Some(Change::Mask {
                slot,
                // The numbers are clamped rather than refused: unlike a
                // transition's start, every value outside the range has one
                // sensible reading — a front past the end has arrived, and one
                // before the start has not. `Mask::new` is where that lives, so
                // a record and a key press cannot disagree about it.
                mask: Mask::new(kind, *angle, *position, *softness),
            }))
        }
        Record::Transition {
            slot,
            control,
            to,
            start,
            beats,
            curve,
        } => {
            let slot = in_range(*slot)?;
            let control = Control::from_name(control).ok_or_else(|| {
                format!(
                    "transition control `{control}` — expected {}",
                    Control::ALL
                        .iter()
                        .map(|c| c.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            let curve = Curve::parse(curve).ok_or_else(|| {
                format!(
                    "transition curve `{curve}` — expected {}",
                    karakuri_engine::binding::CURVES
                        .iter()
                        .map(|c| c.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            // **The numbers, not only the names.** A `start` that is not a
            // number is a control pinned forever — `finished` is never true
            // past it — and a negative duration is a move that ends before it
            // begins. The engine clamps both as a backstop; this is where an
            // operator can be told, which is the whole reason a decode reports
            // rather than rejects.
            if !start.is_finite() {
                return Err(format!("transition start `{start}` is not a position"));
            }
            if !(beats.is_finite() && *beats >= 0.0) {
                return Err(format!(
                    "transition length `{beats}` — expected a number of beats, or 0 for a cut"
                ));
            }
            if !to.is_finite() {
                return Err(format!("transition to `{to}` is not a value"));
            }
            Ok(Some(Change::Transition {
                slot,
                control,
                to: *to,
                start: *start,
                beats: *beats,
                curve,
            }))
        }
        Record::Preview { slot } => Ok(Some(Change::Preview {
            slot: slot.map(in_range).transpose()?,
        })),
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
                mask_record(2, Mask::new(MaskKind::Linear, 1.5, 0.25, 0.1)),
                Change::Mask {
                    slot: 2,
                    mask: Mask::new(MaskKind::Linear, 1.5, 0.25, 0.1),
                },
            ),
            (
                transition_record(1, Control::Opacity, 0.0, 64.0, 8.0, Curve::Smooth),
                Change::Transition {
                    slot: 1,
                    control: Control::Opacity,
                    to: 0.0,
                    start: 64.0,
                    beats: 8.0,
                    curve: Curve::Smooth,
                },
            ),
            (
                preview_record(Some(2)),
                Change::Preview { slot: Some(2) },
            ),
            (preview_record(None), Change::Preview { slot: None }),
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

    /// **Every mask shape round-trips**, so a shape added to the engine and not
    /// to the wire vocabulary is a layer silently unmasked.
    #[test]
    fn every_mask_shape_has_a_wire_name_that_decodes_back() {
        for kind in MaskKind::ALL {
            let mask = Mask::new(kind, 0.5, 0.75, 0.2);
            assert_eq!(
                change(&mask_record(1, mask), 4).expect("built here"),
                Some(Change::Mask { slot: 1, mask }),
                "{} did not survive its own wire name",
                kind.name()
            );
        }
    }

    /// **Every control and every curve a transition can name round-trips**, so
    /// one added to the engine and not to the wire vocabulary is a move that
    /// fails to decode rather than one that moves the wrong thing.
    #[test]
    fn every_transition_control_and_curve_has_a_wire_name_that_decodes_back() {
        for control in Control::ALL {
            for curve in karakuri_engine::binding::CURVES {
                let record = transition_record(0, control, 1.0, 0.0, 4.0, curve);
                assert_eq!(
                    change(&record, 1).expect("built here"),
                    Some(Change::Transition {
                        slot: 0,
                        control,
                        to: 1.0,
                        start: 0.0,
                        beats: 4.0,
                        curve,
                    }),
                    "{} / {} did not survive its own wire name",
                    control.name(),
                    curve.name()
                );
            }
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

        for (start, beats, to, wanted) in [
            (f64::NAN, 4.0, 1.0, "not a position"),
            (0.0, -4.0, 1.0, "expected a number of beats"),
            (0.0, f64::NAN, 1.0, "expected a number of beats"),
            (0.0, 4.0, f32::NAN, "not a value"),
        ] {
            let record = Record::Transition {
                slot: 0,
                control: "gain".to_string(),
                to,
                start,
                beats,
                curve: "lin".to_string(),
            };
            let message = change(&record, 4).expect_err("a number no move can use");
            assert!(message.contains(wanted), "{start}/{beats}/{to}: {message}");
        }

        let unknown_shape = Record::Mask {
            slot: 0,
            kind: "diagonal".to_string(),
            angle: 0.0,
            position: 1.0,
            softness: 0.0,
        };
        let message = change(&unknown_shape, 4).expect_err("`diagonal` is not a shape");
        assert!(message.contains("diagonal"), "{message}");
        assert!(message.contains("radial"), "{message}");

        let unknown_control = Record::Transition {
            slot: 0,
            control: "residency".to_string(),
            to: 1.0,
            start: 0.0,
            beats: 4.0,
            curve: "lin".to_string(),
        };
        let message = change(&unknown_control, 4).expect_err("`residency` is not a control");
        assert!(message.contains("residency"), "{message}");
        assert!(message.contains("opacity"), "{message}");

        let unknown_curve = Record::Transition {
            slot: 0,
            control: "gain".to_string(),
            to: 1.0,
            start: 0.0,
            beats: 4.0,
            curve: "bezier".to_string(),
        };
        let message = change(&unknown_curve, 4).expect_err("`bezier` is not a curve");
        assert!(message.contains("bezier"), "{message}");
        assert!(message.contains("smooth"), "{message}");

        let unknown_mode = Record::Blend {
            slot: 0,
            mode: "screen".to_string(),
        };
        let message = change(&unknown_mode, 4).expect_err("`screen` is not a mode here");
        assert!(message.contains("screen"), "{message}");
        assert!(message.contains("over"), "{message}");
    }

    /// **`None` is the mix and is not a slot**, so it survives the range check
    /// that every other slot-bearing record goes through rather than being
    /// caught by it. A sentinel index would have made "the mix" and "slot 255"
    /// the same line on the wire.
    #[test]
    fn a_preview_of_the_mix_is_not_a_slot_out_of_range() {
        assert_eq!(
            change(&preview_record(None), 1).expect("the mix is always available"),
            Some(Change::Preview { slot: None })
        );
        // And a real slot past the deck still is.
        let message = change(&preview_record(Some(4)), 4).expect_err("slot 4 of a deck of 4");
        assert!(message.contains("slots 0-3"), "{message}");
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
