use karakuri_engine::binding::Curve;
use karakuri_engine::deck::{Blend, Mask, MaskKind, Residency};
use karakuri_engine::master::{Cut, SlotSpec};
use karakuri_engine::transition::Control;
use karakuri_engine::transport::Sync;
use karakuri_engine::Look;
use karakuri_store::record::{DeckSlot, Record};

use super::translate::*;

/// Decoded representation of a mix record for engine consumption.
#[derive(Clone, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub enum Change {
    Gain {
        slot: usize,
        value: f32,
    },
    /// The fader. Distinct from Gain according to the slot's blend mode.
    Opacity {
        slot: usize,
        value: f32,
    },
    Blend {
        slot: usize,
        mode: Blend,
    },
    Mask {
        slot: usize,
        mask: Mask,
    },
    /// A scheduled transition parameter update over musical time.
    Transition {
        slot: usize,
        control: Control,
        to: f32,
        start: f64,
        beats: f64,
        curve: Curve,
    },
    /// Selects the active renderer index of a slot at a musical instant.
    Select {
        slot: usize,
        renderer: usize,
        start: f64,
    },
    Residency {
        slot: usize,
        level: Residency,
    },
    /// Parameter modification on a playing slot, expanded into individual component writes (ADR-0268).
    Ride {
        slot: usize,
        writes: Vec<karakuri_engine::ParamWrite>,
    },
    /// Parameter modulation source binding or removal (`None`).
    Source {
        slot: usize,
        layer: karakuri_ir::Kind,
        index: Option<u32>,
        key: String,
        /// `None` indicates unbinding/clearing the modulation source.
        binding: Option<karakuri_engine::binding::Binding>,
    },
    /// Node authority designation per `(layer, index)`.
    Authority {
        slot: usize,
        layer: karakuri_ir::Kind,
        index: u32,
        authority: karakuri_engine::set::Authority,
    },
    Look(Look),
    /// Master chain entry level across all deck slots (ADR-0224).
    MasterOut(f32),
    /// Described configuration of the master chain (ADR-0340).
    MasterChain(Vec<SlotSpec>),
    /// Slot clock synchronization settings relative to session time.
    Transport {
        slot: usize,
        sync: Sync,
        anchor_bpm: f32,
        scrub_beats: f64,
    },
}

/// Decodes one mix record into an engine change command.
///
/// Returns `Ok(Some(Change))` if decoded, `Ok(None)` if not a mix record,
/// or `Err` with a diagnostic if the record cannot be applied to the current deck.
pub fn change(record: &Record, slot_count: usize) -> Result<Option<Change>, String> {
    // Check slot range against deck slot count.
    let in_range = |slot: DeckSlot| -> Result<usize, String> {
        DeckSlot::new(slot.0, slot_count)
            .map(|slot| slot.index())
            .ok_or_else(|| crate::no_such_slot(slot.index(), slot_count))
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
        Record::Select {
            slot,
            renderer,
            start,
        } => {
            let slot = in_range(*slot)?;
            if !start.is_finite() {
                return Err(format!("selection start `{start}` is not a position"));
            }
            Ok(Some(Change::Select {
                slot,
                renderer: *renderer as usize,
                start: *start,
            }))
        }
        Record::Residency { slot, level } => {
            let slot = in_range(*slot)?;
            let level = parse_residency(level).ok_or_else(|| {
                format!("residency `{level}` — expected {}", residency_wire_names())
            })?;
            Ok(Some(Change::Residency { slot, level }))
        }
        // Decode scalar or vector ride into individual ParamWrites.
        Record::Ride {
            slot,
            at,
            key,
            value,
        } => {
            let slot = in_range(*slot)?;
            let at = at.map(|node| (crate::meta::kind_of(node.layer), node.index));
            let writes = match value {
                karakuri_store::record::Value::Scalar(v) => vec![karakuri_engine::ParamWrite {
                    at,
                    key: key.clone(),
                    value: *v,
                }],
                _ => component_writes(at, key, value.components()),
            };
            Ok(Some(Change::Ride { slot, writes }))
        }
        // Decode parameter modulation source binding or unbinding.
        Record::Source {
            slot,
            layer,
            index,
            key,
            source,
        } => {
            let slot = in_range(*slot)?;
            let binding = match source {
                None => None,
                Some(source) => Some(crate::setfile::binding_from_source(
                    *layer, *index, key, source,
                )?),
            };
            Ok(Some(Change::Source {
                slot,
                layer: crate::meta::kind_of(*layer),
                index: *index,
                key: key.clone(),
                binding,
            }))
        }
        Record::Authority {
            slot,
            at,
            authority,
        } => {
            let slot = in_range(*slot)?;
            let level = karakuri_engine::set::Authority::from_name(authority).ok_or_else(|| {
                format!(
                    "authority `{authority}` — expected {}",
                    karakuri_engine::set::Authority::ALL
                        .iter()
                        .map(|a| a.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            Ok(Some(Change::Authority {
                slot,
                layer: crate::meta::kind_of(at.layer),
                index: at.index,
                authority: level,
            }))
        }
        Record::Transport {
            slot,
            sync,
            anchor_bpm,
            scrub_beats,
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
                scrub_beats: *scrub_beats,
            }))
        }
        Record::Look {
            op,
            exposure,
            white_point,
        } => {
            let op = parse_op(op)
                .ok_or_else(|| format!("tonemap `{op}` — expected {}", op_wire_names()))?;
            Ok(Some(Change::Look(Look {
                op,
                exposure: *exposure,
                white_point: *white_point,
            })))
        }
        Record::MasterOut { value } => Ok(Some(Change::MasterOut(*value))),
        Record::MasterChain(chain) => {
            let mut slots = Vec::with_capacity(chain.slots.len());
            for slot in &chain.slots {
                let cut = match &slot.cut {
                    None => None,
                    Some(word) => Some(Cut::parse(word).ok_or_else(|| {
                        format!(
                            "feedback cut `{word}` — expected {}",
                            Cut::ALL
                                .iter()
                                .map(|c| format!("`{}`", c.name()))
                                .collect::<Vec<_>>()
                                .join(" or ")
                        )
                    })?),
                };
                slots.push(SlotSpec {
                    procedure: slot.procedure.clone(),
                    cut,
                    params: slot.params.clone(),
                });
            }
            Ok(Some(Change::MasterChain(slots)))
        }
        // Non-mix records (ticks, audio, set definitions) are ignored here.
        _ => Ok(None),
    }
}

/// Converts a multi-component parameter value into writes per component key (`key.x`, `key.y`, etc.).
fn component_writes(
    at: Option<(karakuri_ir::Kind, u32)>,
    key: &str,
    values: &[f32],
) -> Vec<karakuri_engine::ParamWrite> {
    values
        .iter()
        .enumerate()
        .map(|(i, value)| karakuri_engine::ParamWrite {
            at,
            key: karakuri_ir::component_key(key, i),
            value: *value,
        })
        .collect()
}

/// Width of a wipe mask's soft transition edge.
pub const MASK_SOFTNESS: f32 = 0.02;
