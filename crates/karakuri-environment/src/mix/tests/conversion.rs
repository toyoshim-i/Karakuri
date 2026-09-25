use super::common::*;

/// Verifies that vocabulary names match engine names across all shared types.
#[test]
fn the_vocabularys_copy_of_a_list_spells_it_the_way_the_store_reads_it() {
    for mode in Blend::ALL {
        assert_eq!(
            blend_mode(mode).name(),
            mode.name(),
            "the vocabulary and the engine spell one blend mode two ways, and \
                 `Record::Blend` carries the name"
        );
    }
    for level in LEVELS {
        assert_eq!(
            residency(level).name(),
            residency_wire_name(level),
            "the vocabulary and the record spell one residency level two ways, and \
                 `Record::Residency` carries the name"
        );
    }
    for mode in Sync::ALL {
        assert_eq!(
            sync(mode).name(),
            mode.name(),
            "the vocabulary and the engine spell one sync mode two ways, and \
                 `Record::Transport` carries the name"
        );
    }
    for op in TONEMAPS {
        assert_eq!(
            tonemap(op).name(),
            op_wire_name(op),
            "the vocabulary and the flag spell one tone map operator two ways, and \
                 `Record::Look` carries the name"
        );
    }
    for level in Authority::ALL {
        assert_eq!(
            authority(level).name(),
            level.name(),
            "the vocabulary and the engine spell one authority level two ways, and \
                 `Record::Authority` carries the name"
        );
    }
    for shape in karakuri_engine::binding::CURVES {
        assert_eq!(
            curve(shape).name(),
            shape.name(),
            "the vocabulary and the engine spell one curve two ways, and \
                 `Record::Transition` carries the name — a fade whose shape does not \
                 decode is a move that fails to replay rather than one that eases \
                 differently"
        );
    }
}

/// Verifies that operation conversion matches the expected records for gestures.
#[test]
fn the_records_the_gestures_built_by_hand_are_what_the_conversion_writes() {
    use karakuri_operation::Operation;

    assert_eq!(
        from_operation(Operation::SetOpacity {
            deck: 2,
            opacity: 0.0,
        }),
        Record::Opacity {
            slot: DeckSlot(2),
            value: 0.0,
        },
        "the silencing a crossfade writes is not what `mix::opacity_record` wrote"
    );
    assert_eq!(
        from_operation(Operation::SetOpacity {
            deck: 1,
            opacity: 1.0,
        }),
        Record::Opacity {
            slot: DeckSlot(1),
            value: 1.0,
        },
        "the opacity a wipe writes is not what `mix::opacity_record` wrote"
    );
    assert_eq!(
        from_operation(Operation::SetBlendMode {
            deck: 1,
            blend: blend_mode(Blend::Over),
        }),
        Record::Blend {
            slot: DeckSlot(1),
            mode: "over".to_string(),
        },
        "the blend mode a wipe forces is not what `mix::blend_record` wrote"
    );
    assert_eq!(
        from_operation(Operation::SetResidency {
            deck: 3,
            residency: residency(Residency::Live),
        }),
        Record::Residency {
            slot: DeckSlot(3),
            level: "live".to_string(),
        },
        "the put-on-air both gestures write is not what `mix::residency_record` wrote"
    );
}

/// Verifies that a quantum of zero schedules the transition immediately.
#[test]
fn a_quantum_of_zero_starts_the_move_on_the_beat_it_was_asked_on() {
    let grid = grid_at(33);
    assert_eq!(grid.beats(), 33.0, "the fixture is not where it says it is");
    // Now, the next beat, the next bar.
    for (quantum, expected) in [(0.0, 33.0), (1.0, 33.0), (4.0, 36.0)] {
        assert_eq!(
            current_transition(&grid, quantum, 4.0, Curve::Smooth, MaskKind::Linear, 0.0).start,
            expected,
            "a quantum of {quantum} on beat 33 scheduled the move at something \
                 other than {expected} — this reading is `quantise` handed over, not a \
                 second grid"
        );
    }
    // And a cut is due the instant it is read, which is what a start of
    // *now* has to mean: `Transition::value_at` and `Selection::due` are
    // both `>=`, so the beat it was asked on belongs to the move.
    let now = current_transition(&grid, 0.0, 0.0, Curve::Smooth, MaskKind::Linear, 0.0);
    assert!(
        karakuri_engine::transition::Transition::new(
            0,
            Control::Opacity,
            1.0,
            0.0,
            now.start,
            now.beats,
            Curve::Smooth,
        )
        .value_at(grid.beats())
        .is_some(),
        "a cut scheduled for now had not begun by now — an operator asking for a \
             cut is waiting for nothing"
    );
}

/// Verifies that scheduled fader transitions decode back to Control::Opacity moves.
#[test]
fn what_the_conversion_schedules_decodes_back_onto_the_fader() {
    for shape in karakuri_engine::binding::CURVES {
        let record = from_operation_reading(
            karakuri_operation::Operation::FadeDeck { deck: 1, to: 0.0 },
            scheduled(33, 4.0, 8.0, shape),
        );
        assert_eq!(
            change(&record, 4).expect("built here"),
            Some(Change::Transition {
                slot: 1,
                control: Control::Opacity,
                to: 0.0,
                start: 36.0,
                beats: 8.0,
                curve: shape,
            }),
            "the fade `written` builds did not decode back as a move on the fader \
                 in {}, so the control or the curve it spells is not the one the engine \
                 reads",
            shape.name()
        );
    }
}

/// Verifies that wipe operations decode into properly sequenced mask setup and position transition.
#[test]
fn what_a_wipe_schedules_decodes_back_onto_the_masks_front() {
    use karakuri_operation_record::Written;

    // The deck arriving is wearing a radial front part way across, which
    // is nothing the wipe asks for: what survives of it is the soft edge
    // and nothing else.
    let worn = Mask::new(MaskKind::Radial, 1.25, 0.4, MASK_SOFTNESS);
    let current = wiping(33, 4.0, 8.0, Curve::Smooth, MaskKind::Linear, 0.75, worn);
    let Written::Records(records) = karakuri_operation_record::written(
        &karakuri_operation::Operation::Wipe { from: 0, to: 1 },
        &current,
    ) else {
        panic!("a wipe with both its readings handed over wrote no records");
    };
    let decoded: Vec<Change> = records
        .iter()
        .map(|record| {
            change(record, 4)
                .expect("built here")
                .expect("every record a wipe writes decodes to a change")
        })
        .collect();
    assert_eq!(
        decoded,
        vec![
            Change::Mask {
                slot: 1,
                mask: Mask::new(MaskKind::Linear, 0.75, 0.4, MASK_SOFTNESS),
            },
            Change::Mask {
                slot: 1,
                mask: Mask::new(MaskKind::Linear, 0.75, 0.0, MASK_SOFTNESS),
            },
            Change::Opacity {
                slot: 1,
                value: 1.0,
            },
            Change::Blend {
                slot: 1,
                mode: Blend::Over,
            },
            Change::Residency {
                slot: 1,
                level: Residency::Live,
            },
            Change::Transition {
                slot: 1,
                control: Control::MaskPosition,
                to: 1.0,
                start: 36.0,
                beats: 8.0,
                curve: Curve::Smooth,
            },
        ],
        "the six records a wipe writes did not decode back as the mask, the front \
             at 0, the opacity, the blend, the put-on-air and one move carrying the \
             front across — a control or a name it spells is not the one the engine \
             reads back"
    );
}

/// Verifies that applying a wipe transition preserves the user's active blend mode
/// on the destination deck. See Principle 0094.
#[test]
fn a_wipe_leaves_the_mode_the_operator_chose_on_the_deck() {
    use karakuri_operation_record::Written;

    let worn = Mask::new(MaskKind::Radial, 1.25, 0.4, MASK_SOFTNESS);
    let current = karakuri_operation_record::Current {
        mix: Some(current_mix(Blend::Max, Residency::Live)),
        ..wiping(33, 4.0, 8.0, Curve::Smooth, MaskKind::Linear, 0.75, worn)
    };
    let Written::Records(records) = karakuri_operation_record::written(
        &karakuri_operation::Operation::Wipe { from: 0, to: 1 },
        &current,
    ) else {
        panic!("a wipe with all three of its readings handed over wrote no records");
    };
    let decoded: Vec<Change> = records
        .iter()
        .map(|record| {
            change(record, 4)
                .expect("built here")
                .expect("every record a wipe writes decodes to a change")
        })
        .collect();
    assert_eq!(
        decoded,
        vec![
            Change::Mask {
                slot: 1,
                mask: Mask::new(MaskKind::Linear, 0.75, 0.4, MASK_SOFTNESS),
            },
            Change::Mask {
                slot: 1,
                mask: Mask::new(MaskKind::Linear, 0.75, 0.0, MASK_SOFTNESS),
            },
            Change::Opacity {
                slot: 1,
                value: 1.0,
            },
            Change::Transition {
                slot: 1,
                control: Control::MaskPosition,
                to: 1.0,
                start: 36.0,
                beats: 8.0,
                curve: Curve::Smooth,
            },
        ],
        "a wipe onto a deck already at `max` and already live decoded to something \
             other than the mask, the front at 0, the opacity and the move — a \
             `Change::Blend` here is the operator's mode being taken back by a gesture \
             that did not have to touch it"
    );
}

/// Verifies that operation conversion for fade_slot and cycle_renderer matches hand-built records.
#[test]
fn the_moves_the_keys_built_by_hand_are_what_the_conversion_writes() {
    use karakuri_operation::Operation;
    use karakuri_operation_record::{written, Written};

    // Beat 33, the next bar, over four beats, eased — which is what the
    // `f` key with the console's own defaults asked for.
    let current = scheduled(33, 4.0, 4.0, Curve::Smooth);
    assert_eq!(
        from_operation_reading(Operation::FadeDeck { deck: 2, to: 0.0 }, current.clone()),
        Record::Transition {
            slot: DeckSlot(2),
            control: "opacity".to_string(),
            to: 0.0,
            start: 36.0,
            beats: 4.0,
            curve: "smooth".to_string(),
        },
        "the move a fade writes is not what `Live::fade_slot` wrote"
    );

    let Written::Records(records) = written(&Operation::Crossfade { from: 0, to: 1 }, &current)
    else {
        panic!("a crossfade with its settings read wrote no records");
    };
    assert_eq!(
        records,
        vec![
            Record::Opacity {
                slot: DeckSlot(1),
                value: 0.0
            },
            Record::Residency {
                slot: DeckSlot(1),
                level: "live".to_string()
            },
            Record::Transition {
                slot: DeckSlot(0),
                control: "opacity".to_string(),
                to: 0.0,
                start: 36.0,
                beats: 4.0,
                curve: "smooth".to_string(),
            },
            Record::Transition {
                slot: DeckSlot(1),
                control: "opacity".to_string(),
                to: 1.0,
                start: 36.0,
                beats: 4.0,
                curve: "smooth".to_string(),
            },
        ],
        "the four records a crossfade writes are not what `Live::crossfade` and \
             `Live::fade_slot` wrote between them"
    );

    assert_eq!(
        from_operation_reading(
            Operation::SelectRenderer {
                deck: 3,
                renderer: 1
            },
            current
        ),
        Record::Select {
            slot: DeckSlot(3),
            renderer: 1,
            start: 36.0,
        },
        "the selection `r` writes is not what `mix::select_record` wrote"
    );
}
