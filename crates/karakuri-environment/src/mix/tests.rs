use super::*;

/// **The two copies of every list, checked against each other — and this
/// is the only place in the workspace where that can happen.**
///
/// `karakuri-operation` owns its own `BlendMode`, `Residency`, `Sync`,
/// `Tonemap` and `Authority` because a vocabulary that refuses to name a
/// value cannot say
/// *set blend to over*, and the cost is stated rather than hidden: they
/// are third spellings of lists the engine and the store already hold
/// (P-0090, ADR-0180). A record carries the **name**, so a level spelled
/// `prime` here and `priming` there is a record that decodes to a refusal
/// on replay and moves nothing in the mix — a failure that would show up
/// as a session replaying differently and nowhere earlier.
///
/// `karakuri-operation` cannot check this (it has no engine, by charter)
/// and neither can `karakuri-operation-record` (it has no engine either,
/// deliberately). This package depends on both, so this is where the two
/// lists meet and where they are made to agree.
///
/// **Both directions per value, over the engine's own lists**, so a value
/// added to the engine arrives here as a missing match arm in
/// [`blend_mode`] and its neighbours rather than as a silent extra.
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

/// One record, as `karakuri-operation-record` writes it.
///
/// **The canonical derivation**, and the only one this program has for
/// these now: a test that wants a `Record::Opacity` asks the conversion
/// for one rather than spelling it, so what it round-trips below is what a
/// key press and a mapped pad actually write. Panics rather than returning
/// nothing on an operation that is not one record, because a test silently
/// given no record is a test that checks nothing.
fn from_operation(operation: karakuri_operation::Operation) -> Record {
    from_operation_reading(operation, karakuri_operation_record::Current::default())
}

/// The same, for an operation whose record is not a function of the
/// operation alone. The mask pair is the case: each half writes
/// `Record::Mask` whole, so each needs the other half read back.
fn from_operation_reading(
    operation: karakuri_operation::Operation,
    current: karakuri_operation_record::Current,
) -> Record {
    use karakuri_operation_record::Written;
    match karakuri_operation_record::written(&operation, &current) {
        Written::Records(records) if records.len() == 1 => {
            records.into_iter().next().expect("length just checked")
        }
        other => panic!("`{}` is not one record: {other:?}", operation.title()),
    }
}

/// A reading of the transition settings a surface is holding, for the
/// four operations that schedule a move.
///
/// Through [`current_transition`] rather than by spelling a
/// `karakuri_operation_record::Transition`, which is `from_operation`'s
/// rule one function down: what these tests convert is what a surface
/// hands over, including the curve going through the two copies of that
/// list.
///
/// **The front shape is fixed here and is a wipe's alone.** A fade, a
/// crossfade and a selection have no front, so the fourth setting is a
/// value none of the three reads; [`wiping`] is what hands one over on
/// purpose.
fn scheduled(at: u8, quantum: f64, beats: f64, shape: Curve) -> karakuri_operation_record::Current {
    karakuri_operation_record::Current {
        transition: Some(current_transition(
            &grid_at(at),
            quantum,
            beats,
            shape,
            MaskKind::Linear,
            0.0,
        )),
        ..karakuri_operation_record::Current::default()
    }
}

/// The same, plus the three readings a wipe takes that a fade does not:
/// the front shape the transition row is holding, the mask the deck being
/// wiped in is already wearing, and where that deck already sits in the
/// mix.
///
/// All three go through the functions a surface calls —
/// [`current_transition`], [`current_mask`] and [`current_mix`] — for
/// [`scheduled`]'s reason: what these tests convert is what a program
/// hands over, including the shape crossing `MaskKind`'s two spellings on
/// the way.
///
/// **The slot is at `add` and off air**, so both of the records a wipe
/// writes conditionally are written: the caller that wants the other case
/// is [`a_wipe_leaves_the_mode_the_operator_chose_on_the_deck`], which
/// hands in its own.
fn wiping(
    at: u8,
    quantum: f64,
    beats: f64,
    shape: Curve,
    front: MaskKind,
    angle: f32,
    worn: Mask,
) -> karakuri_operation_record::Current {
    karakuri_operation_record::Current {
        transition: Some(current_transition(
            &grid_at(at),
            quantum,
            beats,
            shape,
            front,
            angle,
        )),
        mask: Some(current_mask(worn)),
        mix: Some(current_mix(Blend::Add, Residency::Allocated)),
        ..karakuri_operation_record::Current::default()
    }
}

/// A session oscillator standing on beat `at`.
///
/// 120 BPM makes a beat half a second, so a step of that length is a beat
/// apiece and the position is exact rather than a float that nearly is —
/// which matters here, because what these tests are about is the instant a
/// move lands on.
fn grid_at(at: u8) -> Oscillator {
    let mut grid = Oscillator::new(120.0);
    grid.advance(at, 0.5);
    grid
}

/// A reading of a mask that is wearing exactly this, for the two
/// operations that need one.
fn reading_of(mask: Mask) -> karakuri_operation_record::Current {
    karakuri_operation_record::Current {
        mask: Some(karakuri_operation_record::Mask {
            kind: wipe_kind(mask.kind()),
            angle: mask.angle(),
            position: mask.position(),
            softness: mask.softness(),
        }),
        ..karakuri_operation_record::Current::default()
    }
}

/// **The records `crossfade` and `wipe` used to build by hand are the ones
/// the conversion writes.**
///
/// The two gestures asked this module for a `Record::Opacity`, a
/// `Record::Blend` and a `Record::Residency` while their own operations
/// were unsettled, because what was owed was the *scheduled move* and
/// never the silencing or the put-on-air. **Both operations are settled
/// now** — a crossfade when the quantum and the length became a reading,
/// a wipe when the front shape followed them and the soft edge turned out
/// to be the deck's — so each gesture is one `operate` call and these
/// three records come out of `written` whole. This is what says neither
/// change of route changed a byte of what they write.
///
/// **Literals on the right-hand side on purpose.** An expectation derived
/// from `written` would assert that `written` equals itself; these are the
/// records the deleted builders produced, spelled out, including the
/// values the two gestures pass — `0.0` on the deck being silenced and
/// `1.0` on the one arriving under a mask.
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

/// **A quantum of zero starts the move on the beat it was asked on**,
/// which is what a surface with no opinion about the grid hands in.
///
/// `karakuri_engine::transition::quantise` documents 0 as *"now"* —
/// *"an operator who wants a cut does not want to wait for the bar"* — and
/// this is what makes that reachable through the conversion rather than
/// only through the engine: the reading carries the instant the session is
/// already at, so `written` has nothing to invent and no grid to consult.
///
/// The three quanta a keyboard offers are asserted together, because what
/// is being checked is that this function is `quantise` and not a second
/// opinion about it.
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

/// **What the conversion schedules decodes back onto the fader**, which is
/// the one wire name in `karakuri-operation-record` with no list behind it.
///
/// `Record::Transition`'s `control` is `karakuri_engine::transition::Control`'s
/// list, the vocabulary owns no copy of it — no operation names a control,
/// because `Operation::FadeDeck` *is* the opacity one — and that crate
/// cannot reach the engine. So the literal it writes is checked here, in
/// the one package that sees both, exactly as the blend and residency
/// spellings one test up are. A fade that decoded to `gain` would move the
/// trim instead of the fader, which under `over` is a deck that dims
/// without ever getting out of the way.
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

/// **What a wipe schedules decodes back onto the mask's front**, which is
/// the second wire name in `karakuri-operation-record` with no list behind
/// it and is checked here for the first one's reason exactly.
///
/// `Operation::Wipe` *is* the mask-position move — no operation names a
/// control — so the crate writes the literal `mask` and cannot reach
/// `karakuri_engine::transition::Control` to check it. A wipe that spelled
/// it `mask-position` would fail to decode and replay as nothing at all,
/// which is the one failure that looks identical to a wipe nobody asked
/// for.
///
/// **The whole gesture is decoded and not only the move**, because a wipe
/// is six records and what makes it a picture is that the mask lands
/// before the move that carries it: the front is at 0 when the transition
/// is scheduled, and the shape it is at 0 in is the transition row's
/// rather than whatever the deck was wearing.
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

/// **A wipe onto a deck the operator has already moved leaves it where
/// they put it**, which is the affordance `m` in front of `c` is, checked
/// where both halves of it can be seen at once.
///
/// `karakuri-operation-record` holds the same statement about the records;
/// this holds it about the **deck**, which is the half that crate cannot
/// see. A wipe under `max` is a wipe *on* rather than a wipe *over* — a
/// different picture and a legitimate one — and the gesture that decides
/// whether it survives is the one this module hands the reading to
/// ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
///
/// **Decoded rather than counted**, because what is at stake is a
/// `Change::Blend` reaching the deck: a record that decodes to *the mode
/// is `over`* is the mode being taken back from the hand that set it,
/// whatever the list it arrived in was.
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

/// **The records `fade_slot` and `cycle_renderer` built by hand are the
/// ones the conversion writes.**
///
/// The two functions are gone: a fade, a crossfade and a renderer
/// selection go through `Live::operate` now that the transition settings
/// are a reading. This is what says the change of route did not change a
/// byte of what they write, on
/// [`the_records_the_gestures_built_by_hand_are_what_the_conversion_writes`]'s
/// terms — literals on the right-hand side, because an expectation derived
/// from `written` would assert that `written` equals itself.
///
/// The crossfade is here whole, and it is the one that could not be
/// checked this way before: it is four records out of one press, and its
/// two halves have to carry the same instant or they are two fades that
/// happen to be near each other.
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

use karakuri_engine::TonemapOp;

/// The round trip, which is the whole claim: what the live path builds is
/// what a replay would decode, for every mix record there is.
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

/// **A knob turn goes operation → record → the writes the deck takes**,
/// which is the whole road this module exists to be the middle of.
///
/// Through `from_operation` and a real serialisation, on the terms every
/// case in this file uses: the claim is that what a surface asked for
/// survives the wire and comes back as writes at the same address, not that
/// two structs in this crate agree.
#[test]
fn a_ride_decodes_into_the_write_a_deck_takes() {
    let record = from_operation(karakuri_operation::Operation::WriteParam {
        deck: 2,
        param: karakuri_operation::ParamAt {
            node: Some(karakuri_operation::NodeAddress {
                layer: karakuri_operation::Layer::L4,
                index: 1,
            }),
            key: "heat".to_string(),
        },
        value: karakuri_operation::ParamValue::Scalar(2.5),
    });
    let line = serde_json::to_string(&record).expect("serialise");
    let decoded: Record = serde_json::from_str(&line).expect("parse");
    assert_eq!(
        change(&decoded, 4).expect("a record this build built"),
        Some(Change::Ride {
            slot: 2,
            writes: vec![karakuri_engine::ParamWrite::at(
                karakuri_ir::Kind::L4,
                1,
                "heat",
                2.5
            )],
        }),
        "through {line}"
    );

    // The wildcard, which has to stay one: a bare key that came back
    // addressed would move one node where the operator moved every node
    // declaring the name.
    let record = from_operation(karakuri_operation::Operation::WriteParam {
        deck: 0,
        param: karakuri_operation::ParamAt {
            node: None,
            key: "exposure".to_string(),
        },
        value: karakuri_operation::ParamValue::Scalar(1.0),
    });
    assert_eq!(
        change(&record, 4).expect("built here"),
        Some(Change::Ride {
            slot: 0,
            writes: vec![karakuri_engine::ParamWrite::everywhere("exposure", 1.0)],
        })
    );
}

/// **An attachment and the taking of it back go operation → record → what
/// the deck does**, which is the road this module is the middle of and the
/// one a replay travels.
///
/// Three things: the binding comes out of the record with the source, the
/// shape and the range the operation named; the take-back comes out with
/// **no** binding, which is what says the two are one record; and the
/// decoder's own diagnostics are reached, because there is one decoder for
/// a live attachment and a Set file's `bind` rather than two.
#[test]
fn an_attachment_decodes_into_the_binding_a_deck_takes_and_a_take_back_into_none() {
    let record = from_operation(karakuri_operation::Operation::AttachSignal {
        deck: 2,
        param: karakuri_operation::BindAt {
            layer: karakuri_operation::Layer::L1,
            index: Some(1),
            key: "turbulence".to_string(),
        },
        signal: "energy".to_string(),
        curve: karakuri_operation::Curve::Pow2,
        range: [0.1, 2.4],
    });
    let line = serde_json::to_string(&record).expect("serialise");
    let decoded: Record = serde_json::from_str(&line).expect("parse");
    let Some(Change::Source {
        slot,
        layer,
        index,
        key,
        binding,
    }) = change(&decoded, 4).expect("a record this build built")
    else {
        panic!("not an attachment, through {line}");
    };
    assert_eq!(
        (slot, layer, index, key.as_str()),
        (2, karakuri_ir::Kind::L1, Some(1), "turbulence")
    );
    let binding = binding.expect("an attachment carries a binding");
    assert_eq!(
        (
            binding.layer,
            binding.index,
            binding.key.as_str(),
            binding.signal.as_str(),
            binding.curve,
            binding.range,
        ),
        (
            karakuri_ir::Kind::L1,
            Some(1),
            "turbulence",
            "energy",
            karakuri_engine::binding::Curve::Pow2,
            [0.1, 2.4],
        ),
        "the binding is not the attachment the operation named"
    );

    // **The take-back is the same record with nothing in it**, and what it
    // has to carry is the address — a decoder that lost the index would
    // take the layer's binding away instead of this node's.
    let record = from_operation(karakuri_operation::Operation::TakeParamBack {
        deck: 2,
        param: karakuri_operation::BindAt {
            layer: karakuri_operation::Layer::L1,
            index: Some(1),
            key: "turbulence".to_string(),
        },
    });
    assert_eq!(
        change(&record, 4).expect("built here"),
        Some(Change::Source {
            slot: 2,
            layer: karakuri_ir::Kind::L1,
            index: Some(1),
            key: "turbulence".to_string(),
            binding: None,
        }),
        "a take-back did not come back as an attachment that is absent"
    );

    // **One decoder, so the `bind` diagnostics are reached.** A tempo is
    // not a `[0, 1]` signal, and a binding to it would sit at the top of
    // its range for the whole run — which is the same sentence a Set file
    // and a `--bind` meet, in `setfile::binding_from_record`.
    let pinned = Record::Source {
        slot: DeckSlot(0),
        layer: karakuri_store::record::Layer::L1,
        index: None,
        key: "radius".to_string(),
        source: Some(karakuri_store::record::Source {
            signal: "bpm".to_string(),
            curve: "lin".to_string(),
            range: [0.0, 1.0],
            noise: None,
        }),
    };
    let refused = change(&pinned, 4).expect_err("a `bpm` source is refused");
    assert!(
        refused.contains("bpm") && refused.contains("beat"),
        "the refusal does not name the source or what to use instead: {refused}"
    );
}

/// **An authority decodes into the level the operation named**, and a word
/// this build does not know reaches a diagnostic rather than a parser that
/// refuses the line — which is why `Record::Authority` carries a `String`.
#[test]
fn an_authority_decodes_into_the_level_it_names() {
    let record = from_operation(karakuri_operation::Operation::SetAuthority {
        deck: 3,
        node: karakuri_operation::NodeAddress {
            layer: karakuri_operation::Layer::L4,
            index: 1,
        },
        authority: karakuri_operation::Authority::Suggesting,
    });
    let line = serde_json::to_string(&record).expect("serialise");
    let decoded: Record = serde_json::from_str(&line).expect("parse");
    assert_eq!(
        change(&decoded, 4).expect("a record this build built"),
        Some(Change::Authority {
            slot: 3,
            layer: karakuri_ir::Kind::L4,
            index: 1,
            authority: Authority::Suggesting,
        }),
        "through {line}"
    );

    let unknown = Record::Authority {
        slot: DeckSlot(0),
        at: karakuri_store::record::NodeAddress {
            layer: karakuri_store::record::Layer::L1,
            index: 0,
        },
        authority: "supervising".to_string(),
    };
    let refused = change(&unknown, 4).expect_err("a level this build has not got");
    assert!(
        refused.contains("supervising") && refused.contains("manual"),
        "the refusal does not say what was asked for or what this build has: {refused}"
    );
}

/// **A wide value is one record and three writes**, under the component
/// keys ADR-0268 made — and the expansion is here rather than at the
/// applier, so the two binaries cannot come to disagree about what
/// `{"value":[…]}` means.
///
/// `karakuri_ir::component_key` is asserted through rather than around:
/// spelling `"glow.x"` here and in the decoder would be two spellings of
/// one address, which is what that function exists to stop.
#[test]
fn a_wide_ride_becomes_one_write_per_component() {
    let record = from_operation(karakuri_operation::Operation::WriteParam {
        deck: 1,
        param: karakuri_operation::ParamAt {
            node: None,
            key: "glow".to_string(),
        },
        value: karakuri_operation::ParamValue::Vec3([0.4, 0.7, 1.0]),
    });
    let line = serde_json::to_string(&record).expect("serialise");
    let decoded: Record = serde_json::from_str(&line).expect("parse");
    assert_eq!(
        change(&decoded, 4).expect("a record this build built"),
        Some(Change::Ride {
            slot: 1,
            writes: vec![
                karakuri_engine::ParamWrite::everywhere(karakuri_ir::component_key("glow", 0), 0.4),
                karakuri_engine::ParamWrite::everywhere(karakuri_ir::component_key("glow", 1), 0.7),
                karakuri_engine::ParamWrite::everywhere(karakuri_ir::component_key("glow", 2), 1.0),
            ],
        }),
        "through {line}"
    );
}

/// **Every residency level round-trips, not just the one a test remembered
/// to name.** A level added to the engine and not to the wire vocabulary
/// would otherwise be a slot silently refusing to change state.
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

/// **Every mask shape round-trips**, so a shape added to the engine and not
/// to the wire vocabulary is a deck slot silently unmasked.
///
/// Through `Operation::SetMaskShape` rather than a record spelled here,
/// which is `from_operation`'s rule and buys the third spelling with it:
/// the engine's `MaskKind`, the vocabulary's `WipeKind` and the wire name
/// all have to agree for this to decode back.
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

/// **Every control and every curve a transition can name round-trips**, so
/// one added to the engine and not to the wire vocabulary is a move that
/// fails to decode rather than one that moves the wrong thing.
///
/// **Spelled here rather than asked of the conversion**, which is the one
/// place in this module that has to be: `written` writes two of the three
/// controls — `opacity` for a fade and `mask` for a wipe — and no
/// operation names a control at all, so a `gain` move has nothing to be
/// converted from. What is checked is the decoder against the engine's own
/// list, and the two literals the conversion does write are checked
/// against it in
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

/// **Every blend mode round-trips**, so a mode added to the engine and not
/// to the wire vocabulary is a deck slot silently composited the wrong way
/// — which under `over` is a slot that was supposed to hide and does not.
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
                scrub_beats: -0.75,
            }),
            "{} did not survive its own wire name",
            sync.name()
        );
    }
}

/// **The conversion writes exactly the transport the engine would engage**,
/// which is the one thing `karakuri-operation-record` cannot check about
/// itself.
///
/// `Transport::engaged` is where engaging a mode is *decided* — the anchor
/// is the session tempo, the scrub is cleared — and it says so in order
/// that *"a caller building a record of the change and a caller applying
/// one agree by construction"*. The record-builder is `written`, and it
/// **cannot call it**: `karakuri-operation-record` depends on
/// `karakuri-operation` and `karakuri-store` and on nothing else, by
/// charter (ADR-0180, ADR-0194), so an engine under it would be a
/// serialiser and a `wgpu` every surface pays for. The agreement is
/// therefore a convention, and this is the place that enforces it
/// (`docs/contributing.md` §4) — the only crate in the workspace that can
/// see the policy and
/// the conversion at once, which is the argument
/// [`the_vocabularys_copy_of_a_list_spells_it_the_way_the_store_reads_it`]
/// makes about the name lists, one field along.
///
/// **The tempos are the ones a session can actually reach**, taken off an
/// oscillator rather than written here, and both of its writers are
/// exercised: `Oscillator::new` for the launch tempo and
/// `Oscillator::correct` for a tracked one. The out-of-range and NaN asks
/// are in the list because they are what the clamp exists for — an
/// oscillator swallows them, so `current_tempo` never reports one, and if
/// it ever did this is what would notice that the record and the engine
/// had come apart on it.
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

/// Every tone map operator likewise, through `look`. `TONEMAPS` is one
/// table now, so this fails if an operator is added to the enum and not to
/// it.
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

/// **Both ends of the master chain decode**, which is the claim that stops
/// a replay rendering a session's master with the level at 1.0 and the
/// chain off.
///
/// They were under this decoder's wildcard arm until 2026-09-09, and the
/// cost of that was exactly this: `karakuri-cli`'s live path and its
/// replay path both go through `change`, so a stream that pulled the out
/// down or turned a trail up came back without either.
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

/// A record this build cannot obey is **reported, not dropped**. Silently
/// ignoring it would leave a session replaying at the wrong gain with
/// nothing said, which is worse than refusing the line.
#[test]
fn a_record_this_build_cannot_obey_says_so_rather_than_vanishing() {
    let unknown_level = Record::Residency {
        slot: DeckSlot(0),
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

    // **And a feedback cut this build has not got**, refused on the tone
    // map operator's terms and for the sharper reason: a default would not
    // report a wrong level, it would silently play the other picture — one
    // echo where the session had a trail.
    let unknown_cut = Record::MasterChain(karakuri_store::record::Chain {
        slots: vec![karakuri_store::record::ChainSlot {
            procedure: "sha256:feedback".into(),
            cut: Some("previous".to_string()),
            params: Default::default(),
        }],
    });
    let message = change(&unknown_cut, 4).expect_err("`previous` is not a cut here");
    assert!(message.contains("previous"), "{message}");
    for cut in Cut::ALL {
        assert!(message.contains(cut.name()), "{message}");
    }

    for (start, beats, to, wanted) in [
        (f64::NAN, 4.0, 1.0, "not a position"),
        (0.0, -4.0, 1.0, "expected a number of beats"),
        (0.0, f64::NAN, 1.0, "expected a number of beats"),
        (0.0, 4.0, f32::NAN, "not a value"),
    ] {
        let record = Record::Transition {
            slot: DeckSlot(0),
            control: "gain".to_string(),
            to,
            start,
            beats,
            curve: "lin".to_string(),
        };
        let message = change(&record, 4).expect_err("a number no move can use");
        assert!(message.contains(wanted), "{start}/{beats}/{to}: {message}");
    }

    // A selection that never lands, for the reason a transition's does not:
    // nothing clears a queued move whose instant cannot arrive.
    for start in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let record = Record::Select {
            slot: DeckSlot(0),
            renderer: 0,
            start,
        };
        let message = change(&record, 4).expect_err("a beat no selection can land on");
        assert!(message.contains("not a position"), "{start}: {message}");
    }

    let unknown_shape = Record::Mask {
        slot: DeckSlot(0),
        kind: "diagonal".to_string(),
        angle: 0.0,
        position: 1.0,
        softness: 0.0,
    };
    let message = change(&unknown_shape, 4).expect_err("`diagonal` is not a shape");
    assert!(message.contains("diagonal"), "{message}");
    assert!(message.contains("radial"), "{message}");

    let unknown_control = Record::Transition {
        slot: DeckSlot(0),
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
        slot: DeckSlot(0),
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
        slot: DeckSlot(0),
        mode: "screen".to_string(),
    };
    let message = change(&unknown_mode, 4).expect_err("`screen` is not a mode here");
    assert!(message.contains("screen"), "{message}");
    assert!(message.contains("over"), "{message}");
}

/// **A slot the deck does not have is caught in the decode**, where there
/// is something to say about it, rather than four frames later in an index.
///
/// **In the words every other surface says it in**, which is the assertion
/// that had to be an `assert_eq!`: this module spelled the refusal itself,
/// as `slot 4: this deck holds slots 0-3` against the keys' `no slot 4:`,
/// and `contains("slots 0-3")` passed under both. See
/// [`crate::no_such_slot`].
#[test]
fn a_slot_past_the_deck_is_refused_with_the_range_it_missed() {
    let message = change(
        &Record::Gain {
            slot: DeckSlot(4),
            value: 1.0,
        },
        4,
    )
    .expect_err("slot 4 of a deck of 4");
    assert_eq!(message, crate::no_such_slot(4, 4));
    // And the boundary either side of it, which is where the off-by-one
    // this shares with the digit keys would live.
    assert!(change(
        &Record::Gain {
            slot: DeckSlot(3),
            value: 1.0
        },
        4
    )
    .is_ok());
    assert!(change(
        &Record::Gain {
            slot: DeckSlot(0),
            value: 1.0
        },
        1
    )
    .is_ok());
    assert!(change(
        &Record::Gain {
            slot: DeckSlot(1),
            value: 1.0
        },
        1
    )
    .is_err());
}

/// A record that is not the mix's is not an error.
/// `karakuri-environment`'s `audio.rs` decodes those, and both decoders see
/// every record a session carries.
#[test]
fn a_record_that_is_not_the_mixs_is_left_alone() {
    assert_eq!(change(&Record::Tick { steps: 1 }, 4), Ok(None));
}
