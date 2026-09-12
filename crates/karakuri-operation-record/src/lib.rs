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
//!   [`Written::Silent`], or [`Written::Owed`].
//! - **Zero GPU Dependency**: Keeps dependencies limited to `karakuri_operation`
//!   and `karakuri_store`.

use karakuri_operation::Operation;
use karakuri_store::record::{DeckSlot, Record};

/// Output look parameters used to complete tone mapping and exposure records.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    pub tonemap: karakuri_operation::Tonemap,
    pub exposure: f32,
    /// White point value required by [`Record::Look`].
    pub white_point: f32,
}

/// Master chain state used to complete master effect records.
#[derive(Debug, Clone, PartialEq)]
pub struct Chain {
    /// Retained frame feedback configuration.
    pub feedback: karakuri_operation::Feedback,
    pub bloom: f32,
    pub rgb_shift: f32,
    /// Active chain slots in record order.
    pub slots: Vec<karakuri_store::record::ChainSlot>,
    /// Procedure content addresses for shipped effects.
    pub shipped: Shipped,
}

/// Content addresses of shipped master procedures.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shipped {
    pub feedback: String,
    pub bloom: String,
    pub rgb_shift: String,
}

impl Chain {
    /// Returns chain slots updated with the given parameter, appending a slot if missing.
    fn moved(
        &self,
        procedure: &str,
        cut: Option<&karakuri_operation::Cut>,
        key: &str,
        value: f32,
    ) -> Vec<karakuri_store::record::ChainSlot> {
        let mut slots = self.slots.clone();
        let at = match slots.iter().position(|s| s.procedure == procedure) {
            Some(at) => at,
            None => {
                slots.push(karakuri_store::record::ChainSlot {
                    procedure: procedure.to_string(),
                    cut: None,
                    params: Default::default(),
                });
                slots.len() - 1
            }
        };
        slots[at].params.insert(key.to_string(), value);
        if let Some(cut) = cut {
            slots[at].cut = Some(cut.name().to_string());
        }
        slots
    }
}

/// Deck mask parameters used to complete mask shape and position records.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mask {
    pub kind: karakuri_operation::WipeKind,
    /// Linear front angle in radians.
    pub angle: f32,
    /// Front position in range `[0.0, 1.0]`.
    pub position: f32,
    /// Softness value required by [`Record::Mask`].
    pub softness: f32,
}

/// Deck transport state used to complete scrub records.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transport {
    pub sync: karakuri_operation::Sync,
    pub anchor_bpm: f32,
    /// Scrub offset in beats.
    pub scrub_beats: f64,
}

/// Transition parameters governing scheduled fades, crossfades, and wipes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    /// Instant the transition lands on, in absolute session beats.
    pub start: f64,
    /// Duration of the transition in beats (zero represents an immediate cut).
    pub beats: f64,
    /// Easing curve applied to the transition.
    pub curve: karakuri_operation::Curve,
    /// Shape kind for wipe fronts.
    pub wipe_kind: karakuri_operation::WipeKind,
    /// Direction angle of linear wipe fronts in radians.
    pub wipe_angle: f32,
}

/// Deck blend and residency state in the engine mix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mix {
    /// Blend mode of the deck relative to underlying layers.
    pub blend: karakuri_operation::BlendMode,
    /// Deck residency status in the active mix.
    pub residency: karakuri_operation::Residency,
}

/// Snapshot of active engine and surface state when an operation arrives.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Current {
    pub look: Option<Look>,
    /// Active master chain configuration.
    pub master_chain: Option<Chain>,
    pub transport: Option<Transport>,
    pub mask: Option<Mask>,
    /// Session tempo in BPM used to anchor sync mode changes.
    pub tempo: Option<f32>,
    /// Transition timing and shape settings for scheduled moves.
    pub transition: Option<Transition>,
    /// Mix status of the deck arriving in a transition.
    pub mix: Option<Mix>,
}

/// Identifies a specific reading required by an operation when missing from [`Current`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// [`Current::look`].
    Look,
    /// [`Current::master_chain`].
    MasterChain,
    /// [`Current::transport`].
    Transport,
    /// [`Current::mask`].
    Mask,
    /// [`Current::tempo`].
    Tempo,
    /// [`Current::transition`].
    Transition,
    /// [`Current::mix`].
    Mix,
}

/// Rationale for why an operation intentionally produces no journal record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Silent {
    /// Modifies surface-local UI state only (e.g. selection, folding).
    Surface,
    /// Read-only query changing no session state.
    Question,
    /// Record is emitted asynchronously when work lands (e.g. disk saves, hotswap).
    OnLanding,
    /// Expressed in project or set files rather than session journal streams.
    NoRecord,
    /// Configures external display sinks without affecting internal render state.
    Published,
}

impl Silent {
    /// Returns a human-readable explanation for why the operation writes no record.
    pub fn why(self) -> &'static str {
        match self {
            Silent::Surface => "it is a surface's own state and writes no record",
            Silent::Question => "it asks rather than changes, and a question writes no record",
            Silent::OnLanding => {
                "its record is written where the work lands, not where it was asked for"
            }
            Silent::NoRecord => "nothing in the session record vocabulary carries it",
            Silent::Published => {
                "it says where a frame goes, and where a frame goes is not part of the frame"
            }
        }
    }
}

/// Explains why an operation cannot currently be converted into a journal record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owed {
    /// Missing required context snapshot in [`Current`].
    NotRead(Reading),
    /// Operation semantics are undecided in the vocabulary definition.
    Undecided,
    /// Operation requires engine-level arithmetic or beat tracking not provided in [`Current`].
    NotSettled,
}

impl Owed {
    /// Returns an explanatory diagnostic describing the missing requirement.
    pub fn why(self) -> &'static str {
        match self {
            Owed::NotRead(Reading::Look) => "the look that is running was not read",
            Owed::NotRead(Reading::MasterChain) => {
                "the master chain that is running was not read"
            }
            Owed::NotRead(Reading::Transport) => {
                "the transport of the deck it names was not read"
            }
            Owed::NotRead(Reading::Mask) => "the mask of the deck it names was not read",
            Owed::NotRead(Reading::Tempo) => "the session tempo its anchor comes from was not read",
            Owed::NotRead(Reading::Transition) => {
                "the transition settings its move is scheduled by were not handed over"
            }
            Owed::NotRead(Reading::Mix) => {
                "the blend mode and residency of the deck it names were not read"
            }
            Owed::Undecided => "what it acts on is an open question in the vocabulary itself",
            Owed::NotSettled => {
                "the record it writes is not a function of values alone, and who supplies the rest is undecided"
            }
        }
    }
}

/// Translation outcome of converting an [`Operation`] into journal records.
#[derive(Debug, Clone, PartialEq)]
pub enum Written {
    /// Emits one or more journal records to append to the session history in sequence.
    Records(Vec<Record>),
    /// Operation intentionally produces no record.
    Silent(Silent),
    /// Operation cannot produce a record given the current context.
    Owed(Owed),
}

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
        Operation::SetFeedback { params } => match &current.master_chain {
            Some(chain) => one(Record::MasterChain(karakuri_store::record::Chain {
                slots: chain.moved(
                    &chain.shipped.feedback,
                    Some(&params.cut),
                    "amount",
                    params.amount,
                ),
            })),
            None => Written::Owed(Owed::NotRead(Reading::MasterChain)),
        },
        Operation::SetBloom { params } => match &current.master_chain {
            Some(chain) => one(Record::MasterChain(karakuri_store::record::Chain {
                slots: chain.moved(&chain.shipped.bloom, None, "amount", params.amount),
            })),
            None => Written::Owed(Owed::NotRead(Reading::MasterChain)),
        },
        Operation::SetRgbShift { params } => match &current.master_chain {
            Some(chain) => one(Record::MasterChain(karakuri_store::record::Chain {
                slots: chain.moved(&chain.shipped.rgb_shift, None, "amount", params.amount),
            })),
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

        // ----- What it schedules, given the surface's settings -------------
        //
        // Scheduled transitions requiring timing and curve from Current::transition.
        Operation::FadeDeck { deck, to } => match current.transition {
            Some(transition) => one(fade(*deck, *to, transition)),
            None => Written::Owed(Owed::NotRead(Reading::Transition)),
        },
        Operation::Crossfade { from, to } => match current.transition {
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
        Operation::SelectRenderer { deck, renderer } => match current.transition {
            Some(transition) => one(Record::Select {
                slot: DeckSlot(*deck),
                renderer: *renderer,
                start: transition.start,
            }),
            None => Written::Owed(Owed::NotRead(Reading::Transition)),
        },
        Operation::Wipe { from: _, to } => match (current.transition, current.mask, current.mix) {
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
        | Operation::SetPatternGrid { .. }
        | Operation::SelectPattern { .. } => Written::Silent(Silent::Surface),

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

#[cfg(test)]
mod tests {
    use super::*;
    use karakuri_operation::{BlendMode, Residency, Sync, Tonemap};

    /// A look nothing else in these tests happens to be: an operator that is
    /// not the first in the list and a white point that is not the default, so
    /// a conversion filling either in from thin air is visible rather than
    /// coincidentally right. A chain of the three shipped procedures with every
    /// value distinct, so a record that copied the wrong one is a failing
    /// assertion rather than a coincidence. The addresses are short stand-ins
    /// for the real content addresses: what this crate does with one is compare
    /// it, never resolve it.
    fn chain() -> Chain {
        Chain {
            feedback: karakuri_operation::Feedback {
                amount: 0.34,
                cut: karakuri_operation::Cut::Exit,
            },
            bloom: 0.6,
            rgb_shift: 0.25,
            slots: vec![
                slot("sha256:feedback", Some("exit"), 0.34),
                slot("sha256:bloom", None, 0.6),
                slot("sha256:rgb_shift", None, 0.25),
            ],
            shipped: Shipped {
                feedback: "sha256:feedback".into(),
                bloom: "sha256:bloom".into(),
                rgb_shift: "sha256:rgb_shift".into(),
            },
        }
    }

    fn slot(procedure: &str, cut: Option<&str>, amount: f32) -> karakuri_store::record::ChainSlot {
        karakuri_store::record::ChainSlot {
            procedure: procedure.to_string(),
            cut: cut.map(str::to_string),
            params: [("amount".to_string(), amount)].into_iter().collect(),
        }
    }

    fn chain_record(slots: Vec<karakuri_store::record::ChainSlot>) -> Record {
        Record::MasterChain(karakuri_store::record::Chain { slots })
    }

    fn look() -> Look {
        Look {
            tonemap: Tonemap::AgX,
            exposure: 0.25,
            white_point: 4.0,
        }
    }

    fn records(written: Written) -> Vec<Record> {
        match written {
            Written::Records(records) => {
                assert!(
                    !records.is_empty(),
                    "`Records` is documented as never empty"
                );
                records
            }
            other => panic!("expected records, got {other:?}"),
        }
    }

    /// The whole reason this is not a `From` impl.
    ///
    /// `-`, `=` and a MIDI control change turn the exposure alone;
    /// `Record::Look` carries the operator and the white point beside it
    /// because a replay reconstructs a session from the record and *"a stream
    /// that set the exposure without saying which operator it applies to would
    /// be describing a look nobody can reconstruct"* (ADR-0192). The operator
    /// is filled in from the look that is running, and this is what says so.
    #[test]
    fn an_exposure_keeps_the_operator_that_is_running() {
        let current = Current {
            look: Some(look()),
            ..Current::default()
        };
        let written = written(&Operation::SetExposure { exposure: 2.5 }, &current);
        assert_eq!(
            records(written),
            vec![Record::Look {
                op: "agx".to_string(),
                exposure: 2.5,
                white_point: 4.0,
            }],
            "an exposure change rewrote the tone map operator or the white point — \
             the record carries all three and only the exposure was asked for"
        );
    }

    /// The other half, and it fails apart from the first: `t` names an operator
    /// and says nothing about the level going into it, so the exposure and the
    /// white point come from the reading.
    #[test]
    fn a_tone_map_keeps_the_exposure_that_is_running() {
        let current = Current {
            look: Some(look()),
            ..Current::default()
        };
        let written = written(
            &Operation::SetTonemap {
                tonemap: Tonemap::Reinhard,
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![Record::Look {
                op: "reinhard".to_string(),
                exposure: 0.25,
                white_point: 4.0,
            }],
            "a tone map change rewrote the exposure or the white point — the record \
             carries all three and only the operator was asked for"
        );
    }

    /// The chain that is running is what fills in the rows nobody pressed,
    /// which is the look pair's claim with one more row in it: a press on the
    /// bloom row must not put the feedback back where a default left it.
    #[test]
    fn a_bloom_press_keeps_the_feedback_and_the_shift_that_are_running() {
        let current = Current {
            master_chain: Some(chain()),
            ..Current::default()
        };
        let written = written(
            &Operation::SetBloom {
                params: karakuri_operation::Bloom { amount: 0.6 },
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![chain_record(vec![
                slot("sha256:feedback", Some("exit"), 0.34),
                slot("sha256:bloom", None, 0.6),
                slot("sha256:rgb_shift", None, 0.25),
            ])],
            "a bloom press rewrote a slot it did not name — the record carries the \
             whole list and only the bloom slot was asked for"
        );
    }

    /// Feedback carries two of the four, and the cut is one of them: the same
    /// amount is a one-frame echo under `mix` and a compounding trail under
    /// `exit`, so a surface that could move the amount without saying the cut
    /// would be asking for a picture it had not named.
    #[test]
    fn a_feedback_press_carries_its_cut_and_keeps_the_other_two_passes() {
        let current = Current {
            master_chain: Some(chain()),
            ..Current::default()
        };
        let written = written(
            &Operation::SetFeedback {
                params: karakuri_operation::Feedback {
                    amount: 0.9,
                    cut: karakuri_operation::Cut::Mix,
                },
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![chain_record(vec![
                slot("sha256:feedback", Some("mix"), 0.9),
                slot("sha256:bloom", None, 0.6),
                slot("sha256:rgb_shift", None, 0.25),
            ])]
        );
    }

    /// And the third row is the other two's arm. Worth its own test because it
    /// is the row whose figure used to be a dash: an amount of zero is a value
    /// that reaches a record, not a row with nothing to say.
    #[test]
    fn an_rgb_shift_press_writes_a_zero_rather_than_nothing() {
        let current = Current {
            master_chain: Some(chain()),
            ..Current::default()
        };
        let written = written(
            &Operation::SetRgbShift {
                params: karakuri_operation::RgbShift { amount: 0.0 },
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![chain_record(vec![
                slot("sha256:feedback", Some("exit"), 0.34),
                slot("sha256:bloom", None, 0.6),
                slot("sha256:rgb_shift", None, 0.0),
            ])]
        );
    }

    /// A row naming a procedure the chain has not got appends a slot, which is
    /// the *for now* in ADR-0340 §7: with a list, *feedback* is a pass that may
    /// not be in the chain, and until the three rows retire the honest answer
    /// to *turn feedback up* is a chain with feedback in it. It lands at the
    /// end, which is where a drop on the chain lands one too.
    #[test]
    fn a_row_naming_a_procedure_the_chain_has_not_got_appends_a_slot() {
        let mut chain = chain();
        chain.slots = vec![slot("sha256:rgb_shift", None, 0.25)];
        let current = Current {
            master_chain: Some(chain),
            ..Current::default()
        };
        let written = written(
            &Operation::SetFeedback {
                params: karakuri_operation::Feedback {
                    amount: 0.5,
                    cut: karakuri_operation::Cut::Mix,
                },
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![chain_record(vec![
                slot("sha256:rgb_shift", None, 0.25),
                slot("sha256:feedback", Some("mix"), 0.5),
            ])],
            "a row naming a procedure the chain has not got wrote a chain without it"
        );
    }

    /// A chain that was not read is said, never defaulted — the look pair's
    /// rule at the row below it. A default chain here would let a bloom press
    /// silently zero a trail somebody set a moment earlier.
    #[test]
    fn a_master_row_with_no_chain_read_is_owed_it_rather_than_given_a_default() {
        for operation in [
            Operation::SetFeedback {
                params: karakuri_operation::Feedback::default(),
            },
            Operation::SetBloom {
                params: karakuri_operation::Bloom::default(),
            },
            Operation::SetRgbShift {
                params: karakuri_operation::RgbShift::default(),
            },
        ] {
            assert_eq!(
                written(&operation, &Current::default()),
                Written::Owed(Owed::NotRead(Reading::MasterChain)),
                "{operation:?} invented a chain nobody read"
            );
        }
    }

    /// A reading that was not taken is said, never defaulted.
    ///
    /// This is the failure ADR-0192 rejected `cc 20 -> exposure aces` for: a
    /// conversion that filled the operator in from a default would have every
    /// exposure nudge silently overwrite a tone map somebody chose a moment
    /// earlier, sixty times a second. `Current::default()` means *I read
    /// nothing*, and the answer to it is a question rather than a record.
    #[test]
    fn a_reading_that_was_not_taken_is_owed_rather_than_guessed() {
        assert_eq!(
            written(
                &Operation::SetExposure { exposure: 2.5 },
                &Current::default()
            ),
            Written::Owed(Owed::NotRead(Reading::Look)),
            "an exposure with no look read came back with a record — which means the \
             operator in it was invented"
        );
        assert_eq!(
            written(
                &Operation::ScrubDeck {
                    deck: 1,
                    beats: 0.25
                },
                &Current::default()
            ),
            Written::Owed(Owed::NotRead(Reading::Transport)),
            "a scrub with no transport read came back with a record — which means the \
             scrub it moved from was invented"
        );
    }

    /// The master out is a function of the operation and nothing else, and this
    /// is the property that keeps it out of the group above.
    ///
    /// It is the arm most likely to be written as a completion by whoever adds
    /// the second thing to the master chain: it sits between the look pair and
    /// the mask pair in every list, and both of those are records written whole
    /// out of an operation that names a part of one. `Record::MasterOut`
    /// carries one number and the operation carries it, so a reading here would
    /// be a value nobody asked about — and a `Current::default()` that answered
    /// `Owed` would make the console's only route to this level a question
    /// printed instead of a level moved.
    ///
    /// And nothing is clamped, which is this crate's rule at `SetGain`:
    /// `Deck::set_out` floors at zero and is deliberately open above 1.0
    /// because the mix is HDR, so a level of 3.0 arrives on disk as 3.0 and the
    /// engine is the one place that range is decided.
    #[test]
    fn a_master_out_is_written_from_the_operation_alone() {
        assert_eq!(
            records(written(
                &Operation::SetMasterOut { out: 0.25 },
                &Current::default()
            )),
            vec![Record::MasterOut { value: 0.25 }],
            "the master out asked for a reading, or wrote something other than the level it \
             was handed — it names no deck and completes no record, so `Current::default()` \
             is everything it needs"
        );
        // The look that is running is beside the point rather than absent, so
        // a conversion that had started reading one would be caught writing a
        // different record here as well as the same one above.
        let current = Current {
            look: Some(look()),
            ..Current::default()
        };
        assert_eq!(
            records(written(&Operation::SetMasterOut { out: 3.0 }, &current)),
            vec![Record::MasterOut { value: 3.0 }],
            "a master out of 3.0 was clamped, or the look that is running reached the record \
             — the level is open above 1.0 and the engine is where that is decided"
        );
    }

    /// A mask nothing else in these tests happens to be: a shape that is not
    /// the default, an angle nobody would reach for, a front part way across
    /// and a soft edge — so a conversion filling any of the three it was not
    /// asked for from thin air is visible rather than coincidentally right.
    fn mask() -> Mask {
        Mask {
            kind: karakuri_operation::WipeKind::Radial,
            angle: 1.25,
            position: 0.4,
            softness: 0.02,
        }
    }

    /// A deck that is nowhere the wipe is about to put it: still at the blend
    /// mode a slot starts in, and not on air.
    ///
    /// So both of the two records a wipe writes conditionally are written
    /// against this fixture, and a wipe is its full six — which is what makes
    /// [`a_wipe_leaves_a_mode_the_operator_chose_and_a_deck_already_on_air`]
    /// the other half of one statement rather than a second subject. A fixture
    /// already under `over` would have hidden the omission behind a record that
    /// says the same thing.
    fn mix() -> Mix {
        Mix {
            blend: karakuri_operation::BlendMode::Add,
            residency: karakuri_operation::Residency::Allocated,
        }
    }

    /// The shape is asked for and the front is kept, which is the mask half of
    /// ADR-0192's argument: `Record::Mask` is written whole and only the shape
    /// was asked for.
    ///
    /// A conversion that put the front back to a default here would send a
    /// running wipe to the start every time somebody chose a different shape.
    #[test]
    fn a_shape_keeps_the_front_where_it_is() {
        let current = Current {
            mask: Some(mask()),
            ..Current::default()
        };
        let written = written(
            &Operation::SetMaskShape {
                deck: 2,
                kind: karakuri_operation::WipeKind::Linear,
                angle: 0.0,
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![Record::Mask {
                slot: DeckSlot(2),
                kind: "linear".to_string(),
                angle: 0.0,
                position: 0.4,
                softness: 0.02,
            }],
            "choosing a shape moved the front or changed the soft edge — the record \
             carries all four and only the shape and its angle were asked for"
        );
    }

    /// The other half, and it fails apart from the first: a control change
    /// carries a position and says nothing about a shape, so the shape and the
    /// angle come from the reading.
    ///
    /// This is the row that exists because a control change can only set — a
    /// conversion that reset the shape here would turn every fader move into a
    /// layer silently unmasked.
    #[test]
    fn a_front_keeps_the_shape_that_is_running() {
        let current = Current {
            mask: Some(mask()),
            ..Current::default()
        };
        let written = written(
            &Operation::SetMaskPosition {
                deck: 2,
                position: 1.0,
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![Record::Mask {
                slot: DeckSlot(2),
                kind: "radial".to_string(),
                angle: 1.25,
                position: 1.0,
                softness: 0.02,
            }],
            "moving the front rewrote the shape, the angle or the soft edge — the record \
             carries all four and only the position was asked for"
        );
    }

    /// A mask that was not read is said, never defaulted.
    ///
    /// Its own test rather than a third assertion beside the look and the
    /// transport, because the failure it names is the mask's: a conversion that
    /// defaulted would answer `none` for a shape nobody chose, and a
    /// `Record::Mask` saying `none` takes the mask off the layer — so a caller
    /// that read nothing would not get a refusal, it would get a wipe silently
    /// undone.
    #[test]
    fn a_mask_that_was_not_read_is_owed_rather_than_defaulted() {
        assert_eq!(
            written(
                &Operation::SetMaskPosition {
                    deck: 1,
                    position: 0.5
                },
                &Current::default()
            ),
            Written::Owed(Owed::NotRead(Reading::Mask)),
            "a front moved with no mask read came back with a record — which means the \
             shape in it was invented"
        );
        assert_eq!(
            written(
                &Operation::SetMaskShape {
                    deck: 1,
                    kind: karakuri_operation::WipeKind::Radial,
                    angle: 0.0,
                },
                &Current::default()
            ),
            Written::Owed(Owed::NotRead(Reading::Mask)),
            "a shape chosen with no mask read came back with a record — which means the \
             front in it was invented"
        );
    }

    /// A scrub moves from where the slot is, which is why it is the one
    /// operation in the vocabulary that is relative: nothing in the instrument
    /// can set a position. The record is absolute, so the conversion is the
    /// addition.
    #[test]
    fn a_scrub_adds_to_the_scrub_the_slot_is_at() {
        let current = Current {
            transport: Some(Transport {
                sync: Sync::Beat,
                anchor_bpm: 128.0,
                scrub_beats: -1.5,
            }),
            ..Current::default()
        };
        let written = written(
            &Operation::ScrubDeck {
                deck: 2,
                beats: 0.25,
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![Record::Transport {
                slot: DeckSlot(2),
                sync: "beat".to_string(),
                anchor_bpm: 128.0,
                scrub_beats: -1.25,
            }],
            "a scrub wrote somewhere other than where the slot was plus what was asked \
             for — an absolute record built from a relative operation has to read the \
             scrub it is moving from"
        );
    }

    /// Engaging a mode is the sibling of a scrub and reads nothing of the slot,
    /// which is what this asserts by giving it a slot to read.
    ///
    /// `ScrubDeck` above adds to the position the deck holds; `SetSync`
    /// replaces the whole transport, because that is what
    /// `karakuri_engine::transport::Transport::engaged` decides engaging a mode
    /// *means* — anchor at the session tempo, scrub cleared, *"a slot brought
    /// back to the grid should be on the grid, not on wherever it was scrubbed
    /// to a song ago"*. So a reading of a slot that is at -1.5 beats against an
    /// anchor of 128 must not leak into a record written at 126.
    #[test]
    fn engaging_a_mode_anchors_at_the_session_tempo_and_clears_the_scrub() {
        let current = Current {
            tempo: Some(126.0),
            // Deliberately present, deliberately different, and deliberately
            // ignored.
            transport: Some(Transport {
                sync: Sync::Free,
                anchor_bpm: 128.0,
                scrub_beats: -1.5,
            }),
            ..Current::default()
        };
        let written = written(
            &Operation::SetSync {
                deck: 2,
                sync: Sync::Beat,
            },
            &current,
        );
        assert_eq!(
            records(written),
            vec![Record::Transport {
                slot: DeckSlot(2),
                sync: "beat".to_string(),
                anchor_bpm: 126.0,
                scrub_beats: 0.0,
            }],
            "engaging a mode wrote something other than the session tempo as the anchor \
             with the scrub cleared — either the tempo was not what the slot was anchored \
             to, or the position it was scrubbed to survived being brought to the grid"
        );
    }

    /// The tempo is a reading and not a default, on
    /// [`a_reading_that_was_not_taken_is_owed_rather_than_guessed`]'s terms
    /// exactly: a conversion that anchored at 120 because that is a common
    /// tempo would put a slot on a grid the room was never on, silently, and a
    /// replay would reproduce it faithfully.
    #[test]
    fn a_sync_mode_with_no_tempo_read_is_owed_rather_than_anchored_at_a_guess() {
        assert_eq!(
            written(
                &Operation::SetSync {
                    deck: 0,
                    sync: Sync::Tempo
                },
                &Current::default()
            ),
            Written::Owed(Owed::NotRead(Reading::Tempo)),
            "a sync mode with no session tempo read came back with a record, which means \
             the tempo it anchored at was invented"
        );
    }

    /// Seven operations need no reading at all, and a caller that has none to
    /// give still gets its record. `karakuri-console`'s panel is exactly that
    /// caller: three faders, no engine, and `Current::default()`.
    #[test]
    fn the_faders_records_need_no_reading() {
        assert_eq!(
            records(written(
                &Operation::SetGain { deck: 3, gain: 2.0 },
                &Current::default()
            )),
            vec![Record::Gain {
                slot: DeckSlot(3),
                value: 2.0
            }]
        );
        assert_eq!(
            records(written(
                &Operation::SetOpacity {
                    deck: 1,
                    opacity: 0.5
                },
                &Current::default()
            )),
            vec![Record::Opacity {
                slot: DeckSlot(1),
                value: 0.5
            }]
        );
        assert_eq!(
            records(written(
                &Operation::SetBlendMode {
                    deck: 0,
                    blend: BlendMode::Over
                },
                &Current::default()
            )),
            vec![Record::Blend {
                slot: DeckSlot(0),
                mode: "over".to_string()
            }],
            "a fader whose record needed a reading would be a console control that \
             cannot be converted without an engine, which is the seam ADR-0185 opened"
        );
    }

    /// An authority converts with no reading, and carries the node it names.
    ///
    /// Its own test rather than a fourth assertion in
    /// [`the_faders_records_need_no_reading`], because what it pins is not the
    /// absence of a reading but the address: this is the only conversion here
    /// that turns a `karakuri_operation::NodeAddress` into a
    /// `karakuri_store::record::NodeAddress`, and `store_layer` is a second
    /// spelling of a list that has to stay in step. A conversion that dropped
    /// the index would put every renderer's authority on the first one; one
    /// that mistranslated the layer would put an L4's on an L1.
    #[test]
    fn an_authority_carries_the_node_it_names_and_needs_no_reading() {
        assert_eq!(
            records(written(
                &Operation::SetAuthority {
                    deck: 2,
                    node: karakuri_operation::NodeAddress {
                        layer: karakuri_operation::Layer::L4,
                        index: 1,
                    },
                    authority: karakuri_operation::Authority::Suggesting,
                },
                &Current::default()
            )),
            vec![Record::Authority {
                slot: DeckSlot(2),
                at: karakuri_store::record::NodeAddress {
                    layer: karakuri_store::record::Layer::L4,
                    index: 1,
                },
                authority: "suggesting".to_string(),
            }],
            "an authority landed on another node, another deck slot or another \
             level than the one it named"
        );

        // A `kind Field` node takes one too, which is the arm most easily lost
        // in a translation: it addresses no node in the rendering sense and its
        // params are still an operator's to ride.
        assert_eq!(
            records(written(
                &Operation::SetAuthority {
                    deck: 0,
                    node: karakuri_operation::NodeAddress {
                        layer: karakuri_operation::Layer::Field,
                        index: 0,
                    },
                    authority: karakuri_operation::Authority::Manual,
                },
                &Current::default()
            )),
            vec![Record::Authority {
                slot: DeckSlot(0),
                at: karakuri_store::record::NodeAddress {
                    layer: karakuri_store::record::Layer::Field,
                    index: 0,
                },
                authority: "manual".to_string(),
            }]
        );
    }

    /// An attachment writes a record, and the take-back is the same record with
    /// nothing in it.
    ///
    /// Both answered `Silent(NoRecord)` until
    /// `docs/adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md`,
    /// and the reason was the one `WriteParam` had: `Record::Bind` is a Set
    /// file's and has no `slot`. So what is asserted here is the deck arriving
    /// in the record — an attachment made on slot 3 that came back saying
    /// nothing about slot 3 is the whole defect back — and that the two
    /// operations write one `t` rather than two, which is what makes them one
    /// fact for the projection to drop.
    #[test]
    fn an_attachment_and_a_take_back_are_one_record_with_and_without_a_source() {
        let attached = records(written(
            &Operation::AttachSignal {
                deck: 3,
                param: karakuri_operation::BindAt {
                    layer: karakuri_operation::Layer::L1,
                    index: Some(2),
                    key: "turbulence".to_string(),
                },
                signal: "energy".to_string(),
                curve: karakuri_operation::Curve::Pow2,
                range: [0.1, 2.4],
            },
            &Current::default(),
        ));
        assert_eq!(
            attached,
            vec![Record::Source {
                slot: DeckSlot(3),
                layer: karakuri_store::record::Layer::L1,
                index: Some(2),
                key: "turbulence".to_string(),
                source: Some(karakuri_store::record::Source {
                    signal: "energy".to_string(),
                    curve: "pow2".to_string(),
                    range: [0.1, 2.4],
                    // **Absent means the default generator and not the absence
                    // of one**, which is what a `bind` naming `noise` and
                    // saying nothing else has always meant. The operation does
                    // not carry a generator, because kind, rate, stream and
                    // octaves describe the *source* rather than the
                    // attachment.
                    noise: None,
                }),
            }],
            "an attachment landed on another node, another deck slot or another shape \
             than the one it named"
        );

        let taken = records(written(
            &Operation::TakeParamBack {
                deck: 3,
                param: karakuri_operation::BindAt {
                    layer: karakuri_operation::Layer::L1,
                    index: Some(2),
                    key: "turbulence".to_string(),
                },
            },
            &Current::default(),
        ));
        assert_eq!(
            taken,
            vec![Record::Source {
                slot: DeckSlot(3),
                layer: karakuri_store::record::Layer::L1,
                index: Some(2),
                key: "turbulence".to_string(),
                source: None,
            }],
            "a take-back is not the attachment's record with its attachment absent"
        );

        // **The address crosses whole, and a wildcard stays a wildcard.** A
        // binding with no index is the layer's — every node of it declaring
        // the key — and an `index` invented here would narrow it to one node
        // silently.
        let wild = records(written(
            &Operation::TakeParamBack {
                deck: 0,
                param: karakuri_operation::BindAt {
                    layer: karakuri_operation::Layer::L4,
                    index: None,
                    key: "exposure".to_string(),
                },
            },
            &Current::default(),
        ));
        assert_eq!(
            wild,
            vec![Record::Source {
                slot: DeckSlot(0),
                layer: karakuri_store::record::Layer::L4,
                index: None,
                key: "exposure".to_string(),
                source: None,
            }]
        );
    }

    /// A knob turn writes a record, and it needs no reading either.
    ///
    /// This operation answered `Silent(NoRecord)` until
    /// `docs/adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md`,
    /// and the reason given was never that a knob is unworthy of one: it was
    /// that `Record::Param` is a Set file's and has no `slot`. So what is
    /// asserted here is the deck arriving in the record — a write on slot 2
    /// that came back saying nothing about slot 2 would be the whole defect
    /// back again.
    #[test]
    fn a_knob_turn_carries_the_deck_the_set_is_playing_in() {
        assert_eq!(
            records(written(
                &Operation::WriteParam {
                    deck: 2,
                    param: karakuri_operation::ParamAt {
                        node: Some(karakuri_operation::NodeAddress {
                            layer: karakuri_operation::Layer::L4,
                            index: 1,
                        }),
                        key: "glow.x".to_string(),
                    },
                    value: karakuri_operation::ParamValue::Scalar(0.4),
                },
                &Current::default()
            )),
            vec![Record::Ride {
                slot: DeckSlot(2),
                at: Some(karakuri_store::record::NodeAddress {
                    layer: karakuri_store::record::Layer::L4,
                    index: 1,
                }),
                key: "glow.x".to_string(),
                value: karakuri_store::record::Value::Scalar(0.4),
            }],
            "a write landed on another node, another deck slot or another value \
             than the one it named"
        );
    }

    /// A bare key crosses as an absence and never as an invented address.
    ///
    /// `ParamAt::node` is `None` for a wildcard, which means it names no layer
    /// either — and `Record::Ride`'s `at` is the same `Option`, which is the
    /// whole reason that record carries a `NodeAddress` rather than
    /// `Record::Param`'s `layer` beside an `index`. A conversion that filled a
    /// layer in here would be writing a placeholder that a reader then has to
    /// be told to ignore, which is exactly the wart `Record::Param` is still
    /// living with.
    ///
    /// And a wide value crosses whole. A `vec3` is one line a person or a model
    /// writes and the reader with the Set in hand expands it, on `param`'s
    /// terms exactly (ADR-0268); nothing here invents `glow.x`.
    #[test]
    fn a_wildcard_write_names_no_node_and_therefore_no_layer() {
        assert_eq!(
            records(written(
                &Operation::WriteParam {
                    deck: 0,
                    param: karakuri_operation::ParamAt {
                        node: None,
                        key: "glow".to_string(),
                    },
                    value: karakuri_operation::ParamValue::Vec3([0.4, 0.7, 1.0]),
                },
                &Current::default()
            )),
            vec![Record::Ride {
                slot: DeckSlot(0),
                at: None,
                key: "glow".to_string(),
                value: karakuri_store::record::Value::Vec3([0.4, 0.7, 1.0]),
            }],
            "a bare key came back addressed, or a wide value came back expanded"
        );
    }

    /// The wire spellings, which are the cost the vocabulary pays.
    ///
    /// `karakuri-operation` owns copies of lists `karakuri-engine` already
    /// holds, and a record carries the *name*: a mode, a level or an operator
    /// spelled differently here from the way the engine reads it back is a
    /// record that decodes to a refusal on replay and to nothing at all in the
    /// mix. The engine is not reachable from this crate, so this asserts the
    /// literals and `karakuri-cli` — the one crate that sees both lists — is
    /// where they are checked against the engine's own.
    #[test]
    fn a_record_carries_the_name_the_store_is_read_back_with() {
        assert_eq!(BlendMode::Add.name(), "add");
        assert_eq!(BlendMode::Over.name(), "over");
        assert_eq!(BlendMode::Max.name(), "max");
        assert_eq!(Residency::Live.name(), "live");
        assert_eq!(Residency::Priming.name(), "priming");
        assert_eq!(Residency::Allocated.name(), "allocated");
        assert_eq!(Sync::Free.name(), "free");
        assert_eq!(Sync::Tempo.name(), "tempo");
        assert_eq!(Sync::Beat.name(), "beat");
        assert_eq!(Tonemap::Clamp.name(), "clamp");
        assert_eq!(Tonemap::Reinhard.name(), "reinhard");
        assert_eq!(Tonemap::Aces.name(), "aces");
        assert_eq!(Tonemap::AgX.name(), "agx");
        assert_eq!(karakuri_operation::Authority::Manual.name(), "manual");
        assert_eq!(
            karakuri_operation::Authority::Suggesting.name(),
            "suggesting"
        );
        assert_eq!(karakuri_operation::Authority::Automatic.name(), "automatic");
        assert_eq!(karakuri_operation::WipeKind::None.name(), "none");
        assert_eq!(karakuri_operation::WipeKind::Linear.name(), "linear");
        assert_eq!(karakuri_operation::WipeKind::Radial.name(), "radial");
    }

    /// A free-running tempo being stated, which closes the gap P-0090 names:
    /// *"`--bpm` exists and the v0.2 vocabulary has no tempo record."* It has
    /// one now, and `Record::Tempo`'s own documentation says what shape a
    /// statement takes rather than a correction — no shift, no confidence.
    #[test]
    fn a_free_run_tempo_is_a_correction_that_corrects_nothing() {
        assert_eq!(
            records(written(
                &Operation::SetFreeRunTempo { bpm: 174.0 },
                &Current::default()
            )),
            vec![Record::Tempo {
                bpm: 174.0,
                shift: 0.0,
                confidence: 0.0,
            }],
            "a stated tempo carried a phase shift or a confidence — it is a statement \
             rather than an estimate, and a shift would move a beat nobody moved"
        );
    }

    /// The transition settings nothing else in these tests happens to be: an
    /// instant that is not zero and not a whole bar, a length that is not the
    /// default and a curve that is not the first in the list — so a conversion
    /// filling any of the three in from thin air is visible rather than
    /// coincidentally right.
    fn transition() -> Transition {
        Transition {
            start: 37.0,
            beats: 6.0,
            curve: karakuri_operation::Curve::Smooth,
            // And a front shape that is neither the mask fixture's nor the
            // first in the list, for the same reason: a wipe that took its
            // shape off the deck instead of off the settings is visible here
            // rather than coincidentally right.
            wipe_kind: karakuri_operation::WipeKind::Linear,
            wipe_angle: 0.75,
        }
    }

    /// A fade lands on the instant and over the length the surface chose, which
    /// is the whole of what settling this conversion decided: the quantum and
    /// the length are `Operation::SetTransition`'s, that operation writes no
    /// record, and they reach the stream through the reading rather than
    /// through the fade.
    ///
    /// Opacity and never gain, which is `Operation::FadeDeck`'s own sentence: a
    /// fade to zero has to silence the deck under every blend mode, and a gain
    /// of zero under `over` is a black card that still covers.
    #[test]
    fn a_fade_lands_on_the_instant_and_the_length_the_surface_chose() {
        let current = Current {
            transition: Some(transition()),
            ..Current::default()
        };
        assert_eq!(
            records(written(&Operation::FadeDeck { deck: 2, to: 0.0 }, &current)),
            vec![Record::Transition {
                slot: DeckSlot(2),
                control: "opacity".to_string(),
                to: 0.0,
                start: 37.0,
                beats: 6.0,
                curve: "smooth".to_string(),
            }],
            "a fade wrote a move on another control, at another instant, over another \
             length or in another shape than the settings it was handed — every one of \
             those four is the surface's and none of them is on the operation"
        );
    }

    /// A crossfade is four records and both halves share the move.
    ///
    /// The order is the picture and not a preference: the arriving deck is
    /// silenced *before* it is put on air, because a deck comes up at full
    /// opacity and going off air does not lower it — putting one on air first
    /// shows it at full immediately, up to a bar before the fade it is supposed
    /// to arrive on. And a fade to something that is not composited is a fade
    /// to black, so it does have to go on air.
    ///
    /// The two moves share a start and a length, which is what makes this one
    /// gesture without being one type. A conversion that read the settings
    /// twice could not be caught by an equality on one record; it is caught by
    /// asserting the pair.
    #[test]
    fn a_crossfade_is_four_records_and_both_halves_share_the_move() {
        let current = Current {
            transition: Some(transition()),
            ..Current::default()
        };
        assert_eq!(
            records(written(&Operation::Crossfade { from: 0, to: 1 }, &current)),
            vec![
                Record::Opacity {
                    slot: DeckSlot(1),
                    value: 0.0,
                },
                Record::Residency {
                    slot: DeckSlot(1),
                    level: "live".to_string(),
                },
                Record::Transition {
                    slot: DeckSlot(0),
                    control: "opacity".to_string(),
                    to: 0.0,
                    start: 37.0,
                    beats: 6.0,
                    curve: "smooth".to_string(),
                },
                Record::Transition {
                    slot: DeckSlot(1),
                    control: "opacity".to_string(),
                    to: 1.0,
                    start: 37.0,
                    beats: 6.0,
                    curve: "smooth".to_string(),
                },
            ],
            "a crossfade wrote something other than the four records its operation \
             names, in another order, or gave its two halves different instants — the \
             arriving deck is silenced before it is put on air, and the two moves are \
             one gesture exactly because they share a start and a length"
        );
    }

    /// A selection is a cut, so it reads the instant and nothing else.
    ///
    /// `Record::Select` has no length and no curve because *"half way to
    /// renderer 2" does not name a picture*, and this is what says the
    /// conversion agrees: the same operation against two readings that differ
    /// in every field but the start writes the same record.
    #[test]
    fn a_selection_is_a_cut_and_reads_the_instant_alone() {
        let select = Operation::SelectRenderer {
            deck: 3,
            renderer: 2,
        };
        let expected = vec![Record::Select {
            slot: DeckSlot(3),
            renderer: 2,
            start: 37.0,
        }];
        let current = Current {
            transition: Some(transition()),
            ..Current::default()
        };
        assert_eq!(
            records(written(&select, &current)),
            expected,
            "a selection landed on another renderer, another deck or another instant \
             than the one it was handed"
        );
        let other = Current {
            transition: Some(Transition {
                start: 37.0,
                beats: 0.0,
                curve: karakuri_operation::Curve::Lin,
                // The front shape a wipe would take, which a selection has no
                // front to apply it to: the fill is here so that this fixture
                // differs from the last one in the two fields the assertion is
                // about and in nothing else.
                ..transition()
            }),
            ..Current::default()
        };
        assert_eq!(
            records(written(&select, &other)),
            expected,
            "the length or the shape of a fade reached a selection's record — a \
             selection is a choice and a choice is a cut"
        );
    }

    /// A cut is an instant that is now and a length of zero, and both halves of
    /// it arrive rather than being invented.
    ///
    /// `karakuri_engine::transition::quantise` documents a quantum of 0 as
    /// *"now"* and hands the beat count straight back, so a surface asking for
    /// a cut has nothing to say that the grid does not already spell: the start
    /// is the beat the session is on. This crate never sees the quantum — that
    /// is `karakuri_environment::mix`'s
    /// `a_quantum_of_zero_starts_the_move_on_the_beat_it_was_asked_on`, in the
    /// crate that owns the grid — and what it must not do is round, floor or
    /// otherwise improve the instant it was handed.
    #[test]
    fn a_cut_is_the_instant_it_was_handed_and_a_length_of_zero() {
        let current = Current {
            // The beat count a session was at, unrounded on purpose: a
            // conversion that quantised anything would move it.
            transition: Some(Transition {
                start: 12.375,
                beats: 0.0,
                curve: karakuri_operation::Curve::Smooth,
                ..transition()
            }),
            ..Current::default()
        };
        assert_eq!(
            records(written(&Operation::FadeDeck { deck: 1, to: 1.0 }, &current)),
            vec![Record::Transition {
                slot: DeckSlot(1),
                control: "opacity".to_string(),
                to: 1.0,
                start: 12.375,
                beats: 0.0,
                curve: "smooth".to_string(),
            }],
            "a cut was moved onto a grid or given a length — a quantum of 0 is `now`, \
             and an operator who asked for a cut is waiting for nothing"
        );
    }

    /// A wipe is six records, in the order the picture needs, and this is the
    /// whole of what settling that conversion decided: the shape its front
    /// takes is the transition row's and arrives beside the instant and the
    /// length, and the soft edge is read off the mask that is running.
    ///
    /// Every field is asserted against a fixture nothing else here is, so a
    /// value taken from the wrong side is visible: the shape and the angle are
    /// [`transition`]'s and *not* [`mask`]'s, and the softness is [`mask`]'s
    /// and is on no surface at all. A conversion that read the shape off the
    /// deck would write `radial` at 1.25 here, which is the losing answer
    /// spelled out as a failure.
    ///
    /// The two `Record::Mask` are not one, and the first is not redundant: it
    /// is `SetMaskShape`'s record — the shape asked for and the front left
    /// where the deck had it — and the second is `SetMaskPosition`'s, restating
    /// that shape with the front at 0. That is the pair `karakuri-cli`'s `c`
    /// wrote through two `operate` calls, and what this holds is that the
    /// change of route did not change a record.
    #[test]
    fn a_wipe_is_a_mask_at_the_front_and_one_move_carrying_it_across() {
        let current = Current {
            transition: Some(transition()),
            mask: Some(mask()),
            mix: Some(mix()),
            ..Current::default()
        };
        assert_eq!(
            records(written(&Operation::Wipe { from: 3, to: 1 }, &current)),
            vec![
                Record::Mask {
                    slot: DeckSlot(1),
                    kind: "linear".to_string(),
                    angle: 0.75,
                    position: 0.4,
                    softness: 0.02,
                },
                Record::Mask {
                    slot: DeckSlot(1),
                    kind: "linear".to_string(),
                    angle: 0.75,
                    position: 0.0,
                    softness: 0.02,
                },
                Record::Opacity {
                    slot: DeckSlot(1),
                    value: 1.0,
                },
                Record::Blend {
                    slot: DeckSlot(1),
                    mode: "over".to_string(),
                },
                Record::Residency {
                    slot: DeckSlot(1),
                    level: "live".to_string(),
                },
                Record::Transition {
                    slot: DeckSlot(1),
                    control: "mask".to_string(),
                    to: 1.0,
                    start: 37.0,
                    beats: 6.0,
                    curve: "smooth".to_string(),
                },
            ],
            "the six records a wipe writes are not the mask, the front, the opacity, \
             the blend, the put-on-air and the move — in that order, about the deck \
             arriving, with the shape off the transition row and the soft edge off the \
             deck"
        );
    }

    /// The deck being covered is read for nothing, which is what makes a
    /// two-deck operation write about one of them.
    ///
    /// Everything a wipe writes is the arriving deck's: the covered one is
    /// revealed away from rather than moved, and a record naming it would be a
    /// change to a deck the gesture does not touch.
    #[test]
    fn a_wipe_writes_about_the_deck_arriving_and_never_the_one_covered() {
        let current = Current {
            transition: Some(transition()),
            mask: Some(mask()),
            mix: Some(mix()),
            ..Current::default()
        };
        for record in records(written(&Operation::Wipe { from: 3, to: 1 }, &current)) {
            let slot = match record {
                Record::Mask { slot, .. }
                | Record::Opacity { slot, .. }
                | Record::Blend { slot, .. }
                | Record::Residency { slot, .. }
                | Record::Transition { slot, .. } => slot,
                other => panic!("a wipe wrote {other:?}, which is not one of its six"),
            };
            assert_eq!(
                slot,
                DeckSlot(1),
                "a wipe wrote a record about slot {slot} — it names two decks and \
                 writes about the one arriving"
            );
        }
    }

    /// A wipe with the settings but no mask is told it is the mask, which is
    /// the second of its two readings and the one that is read off the deck.
    ///
    /// The softness is the value at stake: no operation names one, so a
    /// conversion with no mask in front of it would have to invent a soft edge
    /// for a front somebody else chose — which is
    /// [`a_mask_that_was_not_read_is_owed_rather_than_defaulted`] arriving at
    /// the gesture that moves the front rather than at the two that set it.
    #[test]
    fn a_wipe_with_no_mask_read_is_owed_the_soft_edge_rather_than_given_one() {
        let current = Current {
            transition: Some(transition()),
            ..Current::default()
        };
        assert_eq!(
            written(&Operation::Wipe { from: 0, to: 1 }, &current),
            Written::Owed(Owed::NotRead(Reading::Mask)),
            "a wipe with no mask read came back with something other than the reading \
             it is missing — a default soft edge is a value nobody asked about, written \
             over one somebody may have"
        );
    }

    /// A wipe onto a deck that is already there writes neither the blend mode
    /// nor the put-on-air, which is the affordance `m` in front of `c` is, said
    /// as a test.
    ///
    /// The mode is the operator's: a wipe under `max` — or under `add` — is a
    /// wipe *on* rather than a wipe *over*, a different picture and a
    /// legitimate one, and a gesture that forced `over` every time would take
    /// it back from the hand that chose it
    /// ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    /// The put-on-air is the same shape with nothing at stake but the byte: a
    /// deck already live is told so again.
    ///
    /// Four records rather than six, in the same order. What the list drops it
    /// drops from the middle, and the front is still at 0 before the move that
    /// carries it across — which is the sentence [`Written::Records`] gained
    /// when the wipe stopped being one length.
    #[test]
    fn a_wipe_leaves_a_mode_the_operator_chose_and_a_deck_already_on_air() {
        let current = Current {
            transition: Some(transition()),
            mask: Some(mask()),
            mix: Some(Mix {
                blend: karakuri_operation::BlendMode::Max,
                residency: karakuri_operation::Residency::Live,
            }),
            ..Current::default()
        };
        let written = records(written(&Operation::Wipe { from: 3, to: 1 }, &current));
        assert!(
            !written
                .iter()
                .any(|record| matches!(record, Record::Blend { .. } | Record::Residency { .. })),
            "a wipe onto a deck already under a mode the operator chose and already \
             live wrote a blend or a residency anyway — `m` in front of `c` means \
             nothing if the gesture writes `over` over it: {written:?}"
        );
        assert_eq!(
            written,
            vec![
                Record::Mask {
                    slot: DeckSlot(1),
                    kind: "linear".to_string(),
                    angle: 0.75,
                    position: 0.4,
                    softness: 0.02,
                },
                Record::Mask {
                    slot: DeckSlot(1),
                    kind: "linear".to_string(),
                    angle: 0.75,
                    position: 0.0,
                    softness: 0.02,
                },
                Record::Opacity {
                    slot: DeckSlot(1),
                    value: 1.0,
                },
                Record::Transition {
                    slot: DeckSlot(1),
                    control: "mask".to_string(),
                    to: 1.0,
                    start: 37.0,
                    beats: 6.0,
                    curve: "smooth".to_string(),
                },
            ],
            "a wipe that leaves the mix alone is the mask, the front at 0, the opacity \
             and the move — in the order the six are in, with the two that change \
             nothing missing rather than the rest reordered"
        );
    }

    /// A deck already under `over` is told it is live and nothing else, which
    /// is the pair one at a time rather than together.
    ///
    /// The two conditions are independent and this is what says so: a deck
    /// wearing the mode the wipe wants but sitting off air needs the put-on-air
    /// and nothing else. Five records, and the one that is missing is the one
    /// that would have restated a mode.
    #[test]
    fn a_wipe_writes_the_put_on_air_alone_for_a_deck_already_under_over() {
        let current = Current {
            transition: Some(transition()),
            mask: Some(mask()),
            mix: Some(Mix {
                blend: karakuri_operation::BlendMode::Over,
                residency: karakuri_operation::Residency::Priming,
            }),
            ..Current::default()
        };
        let written = records(written(&Operation::Wipe { from: 3, to: 1 }, &current));
        assert_eq!(
            written.len(),
            5,
            "a wipe onto a deck already under `over` and not yet live is five records \
             — the two conditions are separate and one of them fired: {written:?}"
        );
        assert_eq!(
            written[3],
            Record::Residency {
                slot: DeckSlot(1),
                level: "live".to_string(),
            },
            "the record a wipe writes for a deck already under `over` is not the \
             put-on-air, or it is not where the order puts it: {written:?}"
        );
    }

    /// A wipe with no mix read is owed it rather than given the records it
    /// would have left out, which is [`Current`]'s every-field-optional rule
    /// meeting the one reading taken so that a record can be omitted.
    ///
    /// The failure this catches is quiet in a way the other four are not: a
    /// default of *the deck is at `add` and off air* is a perfectly plausible
    /// `Mix` and a wipe built on it writes six perfectly plausible records —
    /// one of which is a blend mode nobody chose, written over one somebody
    /// did. There is nothing in the stream afterwards that says a reading was
    /// missing, which is why this answers rather than assumes.
    #[test]
    fn a_wipe_with_no_mix_read_is_owed_it_rather_than_writing_over_a_chosen_mode() {
        let current = Current {
            transition: Some(transition()),
            mask: Some(mask()),
            ..Current::default()
        };
        assert_eq!(
            written(&Operation::Wipe { from: 0, to: 1 }, &current),
            Written::Owed(Owed::NotRead(Reading::Mix)),
            "a wipe with the settings and the mask but no mix read came back with \
             something other than the reading it is missing — a default here is a \
             `blend over` for a deck the operator may have put under `add`"
        );
        assert_ne!(
            Owed::NotRead(Reading::Mix).why(),
            Owed::NotRead(Reading::Mask).why(),
            "the two readings a wipe takes off the deck it names say the same sentence, \
             so a caller is told which of them to go and read only by luck"
        );
    }

    /// A surface that handed in no settings is told which reading it forgot,
    /// rather than getting a cut it did not ask for.
    ///
    /// This is [`a_reading_that_was_not_taken_is_owed_rather_than_guessed`] on
    /// the reading that is not read off anything: a default of zero would be a
    /// perfectly plausible `Transition` — a start of 0 is in the past and a
    /// length of 0 is a cut — so a surface that forgot its settings would get
    /// every fade as an instant jump and nothing anywhere would say so. All
    /// four are asserted, because the failure is the conversion's and not one
    /// operation's.
    ///
    /// The wipe is the fourth and is the one that could answer two things. It
    /// reads the settings and the mask, and with neither handed in it names the
    /// settings — the reading a caller is holding rather than one it would have
    /// had to look up.
    #[test]
    fn a_surface_that_handed_in_no_settings_is_told_which_reading_it_forgot() {
        for operation in [
            Operation::FadeDeck { deck: 1, to: 0.0 },
            Operation::Crossfade { from: 0, to: 1 },
            Operation::SelectRenderer {
                deck: 0,
                renderer: 1,
            },
            Operation::Wipe { from: 0, to: 1 },
        ] {
            assert_eq!(
                written(&operation, &Current::default()),
                Written::Owed(Owed::NotRead(Reading::Transition)),
                "`{operation:?}` with no transition settings handed in came back with \
                 something other than the reading it is missing — a default here is a \
                 cut at beat zero, which is a move nobody asked for and a replay would \
                 reproduce faithfully"
            );
        }
        // And the sentence names the settings rather than something missing,
        // which is what `NotRead` is for.
        assert_ne!(
            Owed::NotRead(Reading::Transition).why(),
            Owed::NotSettled.why(),
            "a surface that forgot its settings is told the same thing as one that met \
             a question nobody has answered"
        );
    }

    /// A file is not a record, and the whole arrangement family says so the
    /// same way.
    ///
    /// Saving one writes `arrangements/<name>.arrangement.json` and nothing
    /// into the session stream, which is the answer that is easy to get wrong
    /// in two directions: `Silent::OnLanding` would promise a `Record` that
    /// arrives when the write lands and none ever does, and `Silent::NoRecord`
    /// would call the decision a gap. Asserted for all three members together,
    /// because what makes the answer right is that they are one family — a
    /// restore is a reset with a name in it.
    #[test]
    fn keeping_an_arrangement_writes_a_file_and_no_record() {
        for operation in [
            Operation::ResetArrangement,
            Operation::SaveArrangement {
                name: "four_deck".to_string(),
            },
            Operation::RestoreArrangement {
                name: "four_deck".to_string(),
            },
        ] {
            assert_eq!(
                written(&operation, &Current::default()),
                Written::Silent(Silent::Surface),
                "`{operation:?}` did not answer `Silent(Surface)`. An arrangement is the \
                 console's own state and lives in a fourth place under the store rather than \
                 in the session stream (ADR-0221) — a save writes a file, and a file is not a \
                 record whose timing `OnLanding` could be about, nor a gap `NoRecord` could be \
                 about"
            );
        }
    }

    /// Editing a pattern writes a file's worth of nothing, exactly as keeping
    /// an arrangement does.
    ///
    /// The five answered `Owed(Undecided)` until 2026-09-09, and the objection
    /// at the arm was that `Silent::Surface` *"would call a pattern the
    /// console's own state, where ADR-0227 makes it library data under the
    /// store."* The test above is the refutation: an arrangement is library
    /// data under the store on the same terms and answers `Silent(Surface)`,
    /// and the sentence that pins it transfers word for word. So this asserts
    /// the second family of the same kind, all five together, because what
    /// makes the answer right is that they are one family — a step, a mute, a
    /// target, a mode and a bank are five edits to one pattern.
    ///
    /// What a lane *does* is not silent and is not asserted here: a lane emits
    /// `Operation::SetOpacity` and `Operation::WriteParam`, whose records are
    /// `Record::Opacity` and `Record::Ride`, and those are what a replay reads
    /// back (ADR-0322).
    #[test]
    fn editing_a_pattern_writes_a_file_and_no_record() {
        for operation in [
            Operation::SetStep {
                pattern: 0,
                lane: 0,
                step: 4,
                on: true,
            },
            Operation::SetLaneMute {
                pattern: 0,
                lane: 0,
                muted: true,
            },
            Operation::PointLane {
                pattern: 0,
                target: karakuri_operation::LaneTarget::Fader { deck: 0 },
            },
            Operation::SetPatternGrid {
                pattern: 0,
                grid: karakuri_operation::StepMode::Eighth,
            },
            Operation::SelectPattern { pattern: 1 },
        ] {
            assert_eq!(
                written(&operation, &Current::default()),
                Written::Silent(Silent::Surface),
                "`{operation:?}` did not answer `Silent(Surface)`. A pattern is library data \
                 under the store on the arrangement's terms (ADR-0227, ADR-0320) and what a \
                 lane does reaches the stream as its own writes (ADR-0322) — so editing one \
                 writes a file and no record, which is the arrangement family's sentence and \
                 not a widening of this arm"
            );
        }
    }

    /// The three answers are three different things, and a caller that
    /// collapsed them would tell an operator that folding a bay failed.
    #[test]
    fn nothing_written_is_never_the_same_as_nothing_decided() {
        assert_eq!(
            written(&Operation::SelectDeck { deck: 1 }, &Current::default()),
            Written::Silent(Silent::Surface),
            "selecting a deck is a surface's own state and settled — not a gap"
        );
        assert_eq!(
            written(&Operation::TapBeat, &Current::default()),
            Written::Owed(Owed::NotSettled),
            "a tap moves the beat tracker and cannot be written here — saying it is \
             silent would lose a tap an operator asked for"
        );
        assert_eq!(
            written(
                &Operation::MoveBoundary {
                    boundary: karakuri_operation::Undecided
                },
                &Current::default()
            ),
            Written::Owed(Owed::Undecided),
            "moving a boundary is the vocabulary's own open question and not this crate's"
        );
    }

    /// A walk asks and changes nothing, and it answers here rather than in
    /// `Owed(Undecided)` because the payload it was waiting for arrived: it
    /// names the Set it is a walk of
    /// (`docs/adr/0342-a-walk-names-the-set-it-is-of-and-the-two-rows-beside-it-are-gap.md`).
    ///
    /// Landing is the other row and is not silent in this way.
    /// `Operation::RestoreProcedure` is `Silent(OnLanding)` — a procedure
    /// change whose `Record::Procedure` is written where the swap lands — so a
    /// walk answering `Question` cannot be read as the store's history being
    /// outside the stream.
    #[test]
    fn a_walk_asks_and_a_landing_writes() {
        for set in [None, Some("night01".to_string())] {
            assert_eq!(
                written(&Operation::WalkHistory { set }, &Current::default()),
                Written::Silent(Silent::Question),
                "walking one Set's versions is a listing of the store: it opens no file, \
                 moves no deck, and a question writes no record"
            );
        }
        assert_eq!(
            written(
                &Operation::RestoreProcedure {
                    deck: 0,
                    revision: karakuri_operation::Revision::Picked(
                        "20260908-143052-271_slot0_L4_beat_strokes".to_string()
                    ),
                },
                &Current::default()
            ),
            Written::Silent(Silent::OnLanding),
            "landing a version is a procedure change and its record is written where the \
             swap lands — the walk is the listing it was picked out of"
        );
    }
}
