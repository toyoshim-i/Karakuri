//! Operation-to-Record translation layer for Karakuri.
//!
//! Converts high-level surface [`Operation`] commands into content-addressed
//! journal [`Record`] entries for session persistence and replay.
//!
//! # Architecture and Guarantees
//!
//! - **Contextual Completeness**: Converts operations like `SetExposure` or
//!   `SetMaskShape` into complete records ([`Record::Look`], [`Record::Mask`])
//!   by reading existing engine state via [`Current`].
//! - **Surface Independence**: All surfaces (GUI, CLI, MIDI, MCP) produce
//!   identical journal records for identical actions.
//! - **Deterministic Classification**: [`written`] performs an exhaustive match
//!   over every operation, classifying it into [`Written::Records`],
//!   [`Written::Silent`], [`Written::Owed`], or [`Written::Refused`].
//! - **Refusal Before Records**: a scheduled move on a control an unmuted lane
//!   of the armed pattern holds is refused here, before any record is written,
//!   so that a replay — which runs no sequencer — sees exactly what the live run
//!   did (ADR-0323).
//! - **Zero GPU Dependency**: Keeps dependencies limited to `karakuri_operation`
//!   and `karakuri_store`.

use karakuri_operation::Operation;
use karakuri_store::record::{DeckSlot, Record};

pub mod state;
pub use state::*;

#[cfg(test)]
mod tests;

/// Translates an [`Operation`] and the [`Current`] engine state into journal records.
pub fn written(operation: &Operation, current: &Current) -> Written {
    match operation {
        // ----- What it writes, with no reading at all ----------------------
        //
        // Operations whose journal record carries exactly what the operation carries.
        Operation::SetGain { deck, gain } => one(Record::Gain {
            slot: DeckSlot(*deck),
            value: *gain,
        }),
        Operation::SetOpacity { deck, opacity } => one(Record::Opacity {
            slot: DeckSlot(*deck),
            value: *opacity,
        }),
        Operation::SetMute { deck, mute } => one(Record::Mute {
            slot: DeckSlot(*deck),
            muted: *mute,
        }),
        Operation::SetSolo { deck, solo } => one(Record::Solo {
            slot: DeckSlot(*deck),
            soloed: *solo,
        }),
        Operation::SetOnline { deck, online } => one(Record::Online {
            slot: DeckSlot(*deck),
            online: *online,
        }),
        Operation::SetBlendMode { deck, blend } => one(Record::Blend {
            slot: DeckSlot(*deck),
            mode: blend.name().to_string(),
        }),
        Operation::SetResidency { deck, residency } => one(Record::Residency {
            slot: DeckSlot(*deck),
            level: residency.name().to_string(),
        }),
        Operation::SetMasterOut { out } => one(Record::MasterOut { value: *out }),
        Operation::SetAuthority {
            deck,
            node,
            authority,
        } => one(Record::Authority {
            slot: DeckSlot(*deck),
            at: karakuri_store::record::NodeAddress {
                layer: store_layer(node.layer),
                index: node.index,
            },
            authority: authority.name().to_string(),
        }),
        Operation::WriteParam { deck, param, value } => one(Record::Ride {
            slot: DeckSlot(*deck),
            at: param.node.map(|node| karakuri_store::record::NodeAddress {
                layer: store_layer(node.layer),
                index: node.index,
            }),
            key: param.key.clone(),
            value: store_value(*value),
        }),
        Operation::AttachSignal {
            deck,
            param,
            signal,
            curve,
            range,
        } => one(Record::Source {
            slot: DeckSlot(*deck),
            layer: store_layer(param.layer),
            index: param.index,
            key: param.key.clone(),
            source: Some(karakuri_store::record::Source {
                signal: signal.clone(),
                curve: curve.name().to_string(),
                range: *range,
                noise: None,
            }),
        }),
        Operation::TakeParamBack { deck, param } => one(Record::Source {
            slot: DeckSlot(*deck),
            layer: store_layer(param.layer),
            index: param.index,
            key: param.key.clone(),
            source: None,
        }),
        Operation::SetFreeRunTempo { bpm } => one(Record::Tempo {
            bpm: *bpm,
            shift: 0.0,
            confidence: 0.0,
        }),

        // ----- What it writes, given a reading ----------------------------
        //
        // Operations requiring context from Current to complete the record.
        Operation::SetTonemap { tonemap } => match current.look {
            Some(look) => one(Record::Look {
                op: tonemap.name().to_string(),
                exposure: look.exposure,
                white_point: look.white_point,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Look)),
        },
        Operation::SetExposure { exposure } => match current.look {
            Some(look) => one(Record::Look {
                op: look.tonemap.name().to_string(),
                exposure: *exposure,
                white_point: look.white_point,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Look)),
        },
        // The whole chain, for whichever slot was asked for. An operation
        // names one slot and `Record::MasterChain` carries the list, so the
        // chain that is running fills in the slots the surface did not name. A
        // position the chain has not got is said rather than appended.
        Operation::SetChainParam { at, param } => match &current.master_chain {
            Some(chain) => match chain.set(*at, param) {
                Some(slots) => one(Record::MasterChain(karakuri_store::record::Chain { slots })),
                None => Written::Owed(Owed::NotInChain),
            },
            None => Written::Owed(Owed::NotRead(Reading::MasterChain)),
        },
        Operation::AddChainEffect { procedure, cut } => match &current.master_chain {
            Some(chain) => one(Record::MasterChain(karakuri_store::record::Chain {
                slots: chain.added(procedure, *cut),
            })),
            None => Written::Owed(Owed::NotRead(Reading::MasterChain)),
        },
        Operation::RemoveChainEffect { at } => match &current.master_chain {
            Some(chain) => match chain.removed(*at) {
                Some(slots) => one(Record::MasterChain(karakuri_store::record::Chain { slots })),
                None => Written::Owed(Owed::NotInChain),
            },
            None => Written::Owed(Owed::NotRead(Reading::MasterChain)),
        },
        Operation::SetMaskShape { deck, kind, angle } => match current.mask {
            Some(mask) => one(Record::Mask {
                slot: DeckSlot(*deck),
                kind: kind.name().to_string(),
                angle: *angle,
                position: mask.position,
                softness: mask.softness,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Mask)),
        },
        Operation::SetMaskPosition { deck, position } => match current.mask {
            Some(mask) => one(Record::Mask {
                slot: DeckSlot(*deck),
                kind: mask.kind.name().to_string(),
                angle: mask.angle,
                position: *position,
                softness: mask.softness,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Mask)),
        },
        Operation::ScrubDeck { deck, beats } => match current.transport {
            Some(transport) => one(Record::Transport {
                slot: DeckSlot(*deck),
                sync: transport.sync.name().to_string(),
                anchor_bpm: transport.anchor_bpm,
                scrub_beats: transport.scrub_beats + *beats,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Transport)),
        },
        Operation::SetSync { deck, sync } => match current.tempo {
            Some(bpm) => one(Record::Transport {
                slot: DeckSlot(*deck),
                sync: sync.name().to_string(),
                anchor_bpm: bpm,
                scrub_beats: 0.0,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Tempo)),
        },

        // Scheduled transitions requiring timing and curve from Current::transition.
        // Refuses the move if an unmuted lane in the armed pattern controls the target fader (ADR-0323).
        Operation::FadeDeck { deck, to } => match held_fader(current, *deck) {
            Some(refusal) => Written::Refused(refusal),
            None => match current.transition {
                Some(transition) => one(fade(*deck, *to, transition)),
                None => Written::Owed(Owed::NotRead(Reading::Transition)),
            },
        },
        // Both ends, because a crossfade moves both faders: the deck leaving is
        // asked about first, so a lane on each end names the one going out.
        Operation::Crossfade { from, to } => {
            match held_fader(current, *from).or_else(|| held_fader(current, *to)) {
                Some(refusal) => Written::Refused(refusal),
                None => match current.transition {
                    Some(transition) => Written::Records(vec![
                        Record::Opacity {
                            slot: DeckSlot(*to),
                            value: 0.0,
                        },
                        Record::Residency {
                            slot: DeckSlot(*to),
                            level: karakuri_operation::Residency::Live.name().to_string(),
                        },
                        fade(*from, 0.0, transition),
                        fade(*to, 1.0, transition),
                    ]),
                    None => Written::Owed(Owed::NotRead(Reading::Transition)),
                },
            }
        }
        Operation::SelectRenderer { deck, renderer } => match current.transition {
            Some(transition) => one(Record::Select {
                slot: DeckSlot(*deck),
                renderer: *renderer,
                start: transition.start,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Transition)),
        },
        // The arriving deck's opacity is brought to 1.0; refused if held by an active pattern lane.
        Operation::Wipe { from: _, to } => match held_fader(current, *to) {
        Some(refusal) => Written::Refused(refusal),
        None => match (current.transition, current.mask, current.mix) {
            (Some(transition), Some(mask), Some(mix)) => {
                let mut records = vec![
                    Record::Mask {
                        slot: DeckSlot(*to),
                        kind: transition.wipe_kind.name().to_string(),
                        angle: transition.wipe_angle,
                        position: mask.position,
                        softness: mask.softness,
                    },
                    Record::Mask {
                        slot: DeckSlot(*to),
                        kind: transition.wipe_kind.name().to_string(),
                        angle: transition.wipe_angle,
                        position: 0.0,
                        softness: mask.softness,
                    },
                    Record::Opacity {
                        slot: DeckSlot(*to),
                        value: 1.0,
                    },
                ];
                if mix.blend == karakuri_operation::BlendMode::Add {
                    records.push(Record::Blend {
                        slot: DeckSlot(*to),
                        mode: karakuri_operation::BlendMode::Over.name().to_string(),
                    });
                }
                if mix.residency != karakuri_operation::Residency::Live {
                    records.push(Record::Residency {
                        slot: DeckSlot(*to),
                        level: karakuri_operation::Residency::Live.name().to_string(),
                    });
                }
                records.push(Record::Transition {
                    slot: DeckSlot(*to),
                    control: MASK.to_string(),
                    to: 1.0,
                    start: transition.start,
                    beats: transition.beats,
                    curve: transition.curve.name().to_string(),
                });
                Written::Records(records)
            }
            (None, _, _) => Written::Owed(Owed::NotRead(Reading::Transition)),
            (_, None, _) => Written::Owed(Owed::NotRead(Reading::Mask)),
            (_, _, None) => Written::Owed(Owed::NotRead(Reading::Mix)),
        },
        },

        // ----- Owed: the tracker ------------------------------------------
        //
        // Operations requiring beat tracker arithmetic not supplied as pure values.
        Operation::TapBeat | Operation::ScaleGrid { .. } => Written::Owed(Owed::NotSettled),

        // ----- Owed: open questions in the vocabulary ----------------------
        Operation::MoveBoundary { .. } | Operation::WatchFiles { .. } => {
            Written::Owed(Owed::Undecided)
        }

        // ----- Silent: publishing is not the performance -------------------
        Operation::RouteFrame { .. } => Written::Silent(Silent::Published),

        // ----- Silent: a surface's own state -------------------------------
        Operation::SelectDeck { .. }
        | Operation::SetTransition { .. }
        | Operation::FoldBay { .. }
        | Operation::FoldPane { .. }
        | Operation::Unfold { .. }
        | Operation::Solo { .. }
        | Operation::ResetArrangement
        | Operation::SaveArrangement { .. }
        | Operation::RestoreArrangement { .. }
        | Operation::SelectScope { .. }
        | Operation::SetFavourite { .. }
        | Operation::KeepProcedure { .. }
        | Operation::PointPane { .. }
        | Operation::SizeWindow { .. }
        | Operation::SetStep { .. }
        | Operation::SetLaneMute { .. }
        | Operation::PointLane { .. }
        // `RemoveLane` edits the same pattern the five around it edit, so it
        // answers what they answer: a pattern is library data under the store
        // and a lane's writes are the lane's own record.
        | Operation::RemoveLane { .. }
        | Operation::SetPatternGrid { .. }
        | Operation::SelectPattern { .. }
        | Operation::ClearSolo => Written::Silent(Silent::Surface),

        // ----- Silent: it asks rather than changes -------------------------
        Operation::ListSets { .. }
        | Operation::FilterLibrary { .. }
        | Operation::ReadSet { .. }
        | Operation::ReadProcedure { .. }
        | Operation::WalkHistory { .. }
        | Operation::SwapOutcome => Written::Silent(Silent::Question),

        // ----- Silent: the record is written where the work lands ----------
        Operation::SaveSet { .. }
        | Operation::WriteProcedure { .. }
        | Operation::RestoreProcedure { .. } => Written::Silent(Silent::OnLanding),

        // ----- Silent: the surface holds the state ------------------------
        Operation::KeepCandidate { .. } => Written::Silent(Silent::Surface),

        // ----- Silent: nothing in the session vocabulary carries it --------
        Operation::SetLatencyOffset { .. }
        | Operation::AttachBeatSource { .. }
        | Operation::LoadSet { .. }
        | Operation::LoadProcedure { .. }
        | Operation::SetCompositing { .. }
        | Operation::WireInput { .. }
        | Operation::Publish { .. }
        | Operation::SetProperty { .. }
        | Operation::TransferSet { .. }
        | Operation::RecordSession { .. }
        | Operation::Quit => Written::Silent(Silent::NoRecord),
    }
}

/// Returns a [`Refusal`] if an active pattern lane holds `deck`'s fader (ADR-0323).
fn held_fader(current: &Current, deck: u8) -> Option<Refusal> {
    let lane = current
        .lanes
        .as_ref()?
        .holder(&karakuri_operation::LaneTarget::Fader { deck })?;
    Some(Refusal { lane, deck })
}

/// Wraps a single record into [`Written::Records`].
fn one(record: Record) -> Written {
    Written::Records(vec![record])
}

/// Constructs a scheduled opacity transition record.
fn fade(slot: u8, to: f32, transition: Transition) -> Record {
    Record::Transition {
        slot: DeckSlot(slot),
        control: OPACITY.to_string(),
        to,
        start: transition.start,
        beats: transition.beats,
        curve: transition.curve.name().to_string(),
    }
}

/// Control identifier for opacity transitions.
const OPACITY: &str = "opacity";

/// Control identifier for mask position transitions.
const MASK: &str = "mask";

/// Converts an operation parameter value to its store record equivalent.
fn store_value(value: karakuri_operation::ParamValue) -> karakuri_store::record::Value {
    use karakuri_operation::ParamValue as From;
    use karakuri_store::record::Value as To;
    match value {
        From::Scalar(v) => To::Scalar(v),
        From::Vec2(v) => To::Vec2(v),
        From::Vec3(v) => To::Vec3(v),
    }
}

/// Converts an operation layer kind to its store record equivalent.
fn store_layer(layer: karakuri_operation::Layer) -> karakuri_store::record::Layer {
    use karakuri_operation::Layer as From;
    use karakuri_store::record::Layer as To;
    match layer {
        From::L1 => To::L1,
        From::L2 => To::L2,
        From::L3 => To::L3,
        From::L4 => To::L4,
        From::Field => To::Field,
        From::L5 => To::L5,
    }
}
