use super::common::*;

/// The two copies of every list, checked against each other — and this is the
/// only place in the workspace where that can happen.
///
/// `karakuri-operation` owns its own `BlendMode`, `Residency`, `Sync`,
/// `Tonemap` and `Authority` because a vocabulary that refuses to name a value
/// cannot say *set blend to over*, and the cost is stated rather than hidden:
/// they are third spellings of lists the engine and the store already hold
/// (P-0090, ADR-0180). A record carries the name, so a level spelled `prime`
/// here and `priming` there is a record that decodes to a refusal on replay and
/// moves nothing in the mix — a failure that would show up as a session
/// replaying differently and nowhere earlier.
///
/// `karakuri-operation` cannot check this (it has no engine, by charter) and
/// neither can `karakuri-operation-record` (it has no engine either,
/// deliberately). This package depends on both, so this is where the two lists
/// meet and where they are made to agree.
///
/// Both directions per value, over the engine's own lists, so a value added to
/// the engine arrives here as a missing match arm in [`blend_mode`] and its
/// neighbours rather than as a silent extra.
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
    // Over the engine's `ALL` and not the vocabulary's, which has none —
    // `karakuri_operation::Authority` says so at its `name`, on
    // `WipeKind`'s terms: that constant exists for a map target, and no map
    // line can say a node address. The engine's list is the one this has to
    // be exhaustive over anyway, for the reason stated above.
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
/// The records `crossfade` and `wipe` used to build by hand are the ones the
/// conversion writes.
///
/// The two gestures asked this module for a `Record::Opacity`, a
/// `Record::Blend` and a `Record::Residency` while their own operations were
/// unsettled, because what was owed was the *scheduled move* and never the
/// silencing or the put-on-air. Both operations are settled now — a crossfade
/// when the quantum and the length became a reading, a wipe when the front
/// shape followed them and the soft edge turned out to be the deck's — so each
/// gesture is one `operate` call and these three records come out of `written`
/// whole. This is what says neither change of route changed a byte of what they
/// write.
///
/// Literals on the right-hand side on purpose. An expectation derived from
/// `written` would assert that `written` equals itself; these are the records
/// the deleted builders produced, spelled out, including the values the two
/// gestures pass — `0.0` on the deck being silenced and `1.0` on the one
/// arriving under a mask.
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

/// A quantum of zero starts the move on the beat it was asked on, which is what
/// a surface with no opinion about the grid hands in.
///
/// `karakuri_engine::transition::quantise` documents 0 as *"now"* — *"an
/// operator who wants a cut does not want to wait for the bar"* — and this is
/// what makes that reachable through the conversion rather than only through
/// the engine: the reading carries the instant the session is already at, so
/// `written` has nothing to invent and no grid to consult.
///
/// The three quanta a keyboard offers are asserted together, because what is
/// being checked is that this function is `quantise` and not a second opinion
/// about it.
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

/// What the conversion schedules decodes back onto the fader, which is the one
/// wire name in `karakuri-operation-record` with no list behind it.
///
/// `Record::Transition`'s `control` is `karakuri_engine::transition::Control`'s
/// list, the vocabulary owns no copy of it — no operation names a control,
/// because `Operation::FadeDeck` *is* the opacity one — and that crate cannot
/// reach the engine. So the literal it writes is checked here, in the one
/// package that sees both, exactly as the blend and residency spellings one
/// test up are. A fade that decoded to `gain` would move the trim instead of
/// the fader, which under `over` is a deck that dims without ever getting out
/// of the way.
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

/// What a wipe schedules decodes back onto the mask's front, which is the
/// second wire name in `karakuri-operation-record` with no list behind it and
/// is checked here for the first one's reason exactly.
///
/// `Operation::Wipe` *is* the mask-position move — no operation names a control
/// — so the crate writes the literal `mask` and cannot reach
/// `karakuri_engine::transition::Control` to check it. A wipe that spelled it
/// `mask-position` would fail to decode and replay as nothing at all, which is
/// the one failure that looks identical to a wipe nobody asked for.
///
/// The whole gesture is decoded and not only the move, because a wipe is six
/// records and what makes it a picture is that the mask lands before the move
/// that carries it: the front is at 0 when the transition is scheduled, and the
/// shape it is at 0 in is the transition row's rather than whatever the deck
/// was wearing.
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

/// The records `fade_slot` and `cycle_renderer` built by hand are the ones the
/// conversion writes.
///
/// The two functions are gone: a fade, a crossfade and a renderer selection go
/// through `Live::operate` now that the transition settings are a reading. This
/// is what says the change of route did not change a byte of what they write,
/// on
/// [`the_records_the_gestures_built_by_hand_are_what_the_conversion_writes`]'s
/// terms — literals on the right-hand side, because an expectation derived from
/// `written` would assert that `written` equals itself.
///
/// The crossfade is here whole, and it is the one that could not be checked
/// this way before: it is four records out of one press, and its two halves
/// have to carry the same instant or they are two fades that happen to be near
/// each other.
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
