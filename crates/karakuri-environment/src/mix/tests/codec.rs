use super::common::*;

/// The round trip, which is the whole claim: what the live path builds is what
/// a replay would decode, for every mix record there is.
#[test]
fn every_mix_record_survives_the_json_between() {
    let cases = [
        (
            Record::Gain {
                slot: DeckSlot(2),
                value: 0.75,
            },
            Change::Gain {
                slot: 2,
                value: 0.75,
            },
        ),
        (
            from_operation(karakuri_operation::Operation::SetOpacity {
                deck: 0,
                opacity: 0.25,
            }),
            Change::Opacity {
                slot: 0,
                value: 0.25,
            },
        ),
        (
            from_operation_reading(
                karakuri_operation::Operation::SetMaskShape {
                    deck: 2,
                    kind: karakuri_operation::WipeKind::Linear,
                    angle: 1.5,
                },
                reading_of(Mask::new(MaskKind::Linear, 1.5, 0.25, 0.1)),
            ),
            Change::Mask {
                slot: 2,
                mask: Mask::new(MaskKind::Linear, 1.5, 0.25, 0.1),
            },
        ),
        (
            from_operation_reading(
                karakuri_operation::Operation::FadeDeck { deck: 1, to: 0.0 },
                scheduled(33, 64.0, 8.0, Curve::Smooth),
            ),
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
            from_operation_reading(
                karakuri_operation::Operation::SelectRenderer {
                    deck: 2,
                    renderer: 1,
                },
                // A grid at beat 33 and a quantum of 64, so the instant
                // is one a reader can see was quantised rather than
                // passed through.
                scheduled(33, 64.0, 0.0, Curve::Lin),
            ),
            Change::Select {
                slot: 2,
                renderer: 1,
                start: 64.0,
            },
        ),
        (
            from_operation(karakuri_operation::Operation::SetBlendMode {
                deck: 3,
                blend: blend_mode(Blend::Over),
            }),
            Change::Blend {
                slot: 3,
                mode: Blend::Over,
            },
        ),
        (
            from_operation(karakuri_operation::Operation::SetResidency {
                deck: 1,
                residency: residency(Residency::Priming),
            }),
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
                scrub_beats: 0.0,
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

/// Every residency level round-trips, not just the one a test remembered to
/// name. A level added to the engine and not to the wire vocabulary would
/// otherwise be a slot silently refusing to change state.
#[test]
fn every_residency_level_has_a_wire_name_that_decodes_back() {
    for level in LEVELS {
        let record = from_operation(karakuri_operation::Operation::SetResidency {
            deck: 0,
            residency: residency(level),
        });
        assert_eq!(
            change(&record, 1).expect("built here"),
            Some(Change::Residency { slot: 0, level }),
            "{level:?} did not survive its own wire name"
        );
    }
}

/// Every mask shape round-trips, so a shape added to the engine and not to the
/// wire vocabulary is a deck slot silently unmasked.
///
/// Through `Operation::SetMaskShape` rather than a record spelled here, which
/// is `from_operation`'s rule and buys the third spelling with it: the engine's
/// `MaskKind`, the vocabulary's `WipeKind` and the wire name all have to agree
/// for this to decode back.
#[test]
fn every_mask_shape_has_a_wire_name_that_decodes_back() {
    for kind in MaskKind::ALL {
        let mask = Mask::new(kind, 0.5, 0.75, 0.2);
        let record = from_operation_reading(
            karakuri_operation::Operation::SetMaskShape {
                deck: 1,
                kind: wipe_kind(kind),
                angle: 0.5,
            },
            reading_of(mask),
        );
        assert_eq!(
            change(&record, 4).expect("built here"),
            Some(Change::Mask { slot: 1, mask }),
            "{} did not survive its own wire name",
            kind.name()
        );
    }
}

/// Every control and every curve a transition can name round-trips, so one
/// added to the engine and not to the wire vocabulary is a move that fails to
/// decode rather than one that moves the wrong thing.
///
/// Spelled here rather than asked of the conversion, which is the one place in
/// this module that has to be: `written` writes two of the three controls —
/// `opacity` for a fade and `mask` for a wipe — and no operation names a
/// control at all, so a `gain` move has nothing to be converted from. What is
/// checked is the decoder against the engine's own list, and the two literals
/// the conversion does write are checked against it in
/// [`what_the_conversion_schedules_decodes_back_onto_the_fader`] and
/// [`what_a_wipe_schedules_decodes_back_onto_the_masks_front`].
#[test]
fn every_transition_control_and_curve_has_a_wire_name_that_decodes_back() {
    for control in Control::ALL {
        for curve in karakuri_engine::binding::CURVES {
            let record = Record::Transition {
                slot: DeckSlot(0),
                control: control.name().to_string(),
                to: 1.0,
                start: 0.0,
                beats: 4.0,
                curve: curve.name().to_string(),
            };
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

/// Every blend mode round-trips, so a mode added to the engine and not to the
/// wire vocabulary is a deck slot silently composited the wrong way — which
/// under `over` is a slot that was supposed to hide and does not.
#[test]
fn every_blend_mode_has_a_wire_name_that_decodes_back() {
    for mode in Blend::ALL {
        let record = from_operation(karakuri_operation::Operation::SetBlendMode {
            deck: 2,
            blend: blend_mode(mode),
        });
        assert_eq!(
            change(&record, 4).expect("built here"),
            Some(Change::Blend { slot: 2, mode }),
            "{} did not survive its own wire name",
            mode.name()
        );
    }
}

/// Every sync mode round-trips, so a mode added to the engine and not to the
/// wire vocabulary is a slot silently left free rather than put where the
/// record said.
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
                scrub_beats: -0.75,
            }),
            "{} did not survive its own wire name",
            sync.name()
        );
    }
}

/// The conversion writes exactly the transport the engine would engage, which
/// is the one thing `karakuri-operation-record` cannot check about itself.
///
/// `Transport::engaged` is where engaging a mode is *decided* — the anchor is
/// the session tempo, the scrub is cleared — and it says so in order that *"a
/// caller building a record of the change and a caller applying one agree by
/// construction"*. The record-builder is `written`, and it cannot call it:
/// `karakuri-operation-record` depends on `karakuri-operation` and
/// `karakuri-store` and on nothing else, by charter (ADR-0180, ADR-0194), so an
/// engine under it would be a serialiser and a `wgpu` every surface pays for.
/// The agreement is therefore a convention, and this is the place that enforces
/// it (`docs/contributing.md` §4) — the only crate in the workspace that can
/// see the policy and the conversion at once, which is the argument
/// [`the_vocabularys_copy_of_a_list_spells_it_the_way_the_store_reads_it`]
/// makes about the name lists, one field along.
///
/// The tempos are the ones a session can actually reach, taken off an
/// oscillator rather than written here, and both of its writers are exercised:
/// `Oscillator::new` for the launch tempo and `Oscillator::correct` for a
/// tracked one. The out-of-range and NaN asks are in the list because they are
/// what the clamp exists for — an oscillator swallows them, so `current_tempo`
/// never reports one, and if it ever did this is what would notice that the
/// record and the engine had come apart on it.
#[test]
fn a_sync_mode_writes_exactly_what_the_engine_would_engage() {
    let mut corrected = Oscillator::new(120.0);
    // A wild estimate, which `Oscillator::correct` says is a thing that
    // happens: it lands on the top of the range.
    corrected.correct(9_999.0, 0.0);
    let mut slow = Oscillator::new(f32::NAN);
    slow.correct(-4.0, 0.0);
    let grids = [
        Oscillator::new(120.0),
        Oscillator::new(126.5),
        Oscillator::new(0.0),
        Oscillator::new(f32::NAN),
        Oscillator::new(f32::INFINITY),
        Oscillator::new(4_000.0),
        corrected,
        slow,
    ];
    for grid in grids {
        let tempo = current_tempo(&grid);
        for mode in Sync::ALL {
            let record = from_operation_reading(
                karakuri_operation::Operation::SetSync {
                    deck: 2,
                    sync: sync(mode),
                },
                karakuri_operation_record::Current {
                    tempo: Some(tempo),
                    ..karakuri_operation_record::Current::default()
                },
            );
            assert_eq!(
                record,
                transport_record(2, &Transport::engaged(mode, tempo)),
                "the record `written` writes for `{}` at {tempo} bpm is not the \
                     transport `Transport::engaged` would engage — the conversion and the \
                     engine's policy have drifted, and a replay would put the slot \
                     somewhere the operator's press did not",
                mode.name()
            );
        }
    }
}

/// Every tone map operator likewise, through `look`. `TONEMAPS` is one table
/// now, so this fails if an operator is added to the enum and not to it.
#[test]
fn every_tonemap_operator_has_a_wire_name_that_decodes_back() {
    for op in TONEMAPS {
        let look = Look {
            op,
            exposure: 1.0,
            white_point: 4.0,
        };
        let Some(Change::Look(decoded)) = change(&look_record(&look), 1).expect("built here")
        else {
            panic!("a look record did not decode as a look");
        };
        assert_eq!(
            decoded.op,
            op,
            "{} did not survive its wire name",
            op_wire_name(op)
        );
    }
}

/// Both ends of the master chain decode, which is the claim that stops a replay
/// rendering a session's master with the level at 1.0 and the chain off.
///
/// They were under this decoder's wildcard arm until 2026-09-09, and the cost
/// of that was exactly this: `karakuri-cli`'s live path and its replay path
/// both go through `change`, so a stream that pulled the out down or turned a
/// trail up came back without either.
#[test]
fn both_ends_of_the_master_chain_decode_from_their_records() {
    let Some(Change::MasterOut(level)) =
        change(&Record::MasterOut { value: 0.5 }, 1).expect("built here")
    else {
        panic!("a master out record did not decode as one");
    };
    assert_eq!(level, 0.5);

    // **Every cut, through its wire word**, which is the tone map
    // operator's test one bay along: a cut added to the engine and not to
    // the spelling fails here rather than in a replay.
    for want in Cut::ALL {
        let record = Record::MasterChain(karakuri_store::record::Chain {
            slots: vec![
                karakuri_store::record::ChainSlot {
                    procedure: "sha256:feedback".into(),
                    cut: Some(want.name().to_string()),
                    params: [("amount".to_string(), 0.34)].into_iter().collect(),
                },
                karakuri_store::record::ChainSlot {
                    procedure: "sha256:bloom".into(),
                    cut: None,
                    params: [("amount".to_string(), 0.6)].into_iter().collect(),
                },
            ],
        });
        let Some(Change::MasterChain(slots)) = change(&record, 1).expect("built here") else {
            panic!("a master chain record did not decode as one");
        };
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].procedure, "sha256:feedback");
        assert_eq!(
            slots[0].cut,
            Some(want),
            "`{}` did not survive its wire word",
            want.name()
        );
        assert_eq!(slots[0].params.get("amount"), Some(&0.34));
        // **A slot whose procedure declares no `retains` carries no cut**,
        // and the decode does not invent one: the engine refuses a cut that
        // was not asked for, so a default here would build a chain this
        // build then refuses to install.
        assert_eq!(slots[1].cut, None);
        assert_eq!(slots[1].params.get("amount"), Some(&0.6));
    }

    // **An empty list is a chain and not an absence.** The default chain is
    // empty, so this is the record a session writes when the last slot is
    // taken out — and reading it as *nothing to do* would leave the chain
    // that was running on air.
    let Some(Change::MasterChain(slots)) = change(
        &Record::MasterChain(karakuri_store::record::Chain::default()),
        1,
    )
    .expect("built here") else {
        panic!("an empty master chain record did not decode as one");
    };
    assert!(slots.is_empty());
}
