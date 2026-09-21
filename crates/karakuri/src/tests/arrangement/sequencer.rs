use super::*;

/// Verifies that Sequencer bay operations update the targeted pattern bank state.
#[test]
fn a_sequencer_press_moves_the_pattern_it_names() {
    let mut banks = demonstration_banks();
    let mut playhead = karakuri_pattern::Playhead::default();
    assert!(
        banks.pattern().lanes()[0].muted(),
        "a run starts with the lane muted, so nothing writes deck A's fader until a hand asks"
    );
    // The mute, taken back.
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &View::new(Room::Day),
        &Operation::SetLaneMute {
            pattern: 0,
            lane: 0,
            muted: false,
        }
    )
    .is_some());
    assert!(!banks.pattern().lanes()[0].muted());
    // A step, set rather than flipped.
    for on in [true, true, false] {
        assert!(sequenced(
            &mut banks,
            &mut playhead,
            &View::new(Room::Day),
            &Operation::SetStep {
                pattern: 0,
                lane: 0,
                step: 4,
                on,
            }
        )
        .is_some());
        assert_eq!(
            banks.pattern().lanes()[0].slot_on(4),
            on,
            "a press asks for a state, so asking twice for the same one leaves it there"
        );
    }
    // A slot the mode press must not touch, and it is slot 5 — an odd one,
    // which an eighth does not read at all and which is therefore the slot
    // a store sized to the count would have thrown away.
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &View::new(Room::Day),
        &Operation::SetStep {
            pattern: 0,
            lane: 0,
            step: 5,
            on: true,
        }
    )
    .is_some());
    // The mode, which changes a reading and forgets where the playhead
    // was, because the index it remembered is about a reading that has
    // gone.
    playhead.advance(banks.pattern(), 0.0);
    assert!(playhead.at().is_some());
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &View::new(Room::Day),
        &Operation::SetPatternGrid {
            pattern: 0,
            grid: karakuri_operation::StepMode::Eighth,
        }
    )
    .is_some());
    assert_eq!(banks.pattern().mode().count(), 8);
    assert_eq!(
        playhead.at(),
        None,
        "a mode press is the same bar at another width, so the next poll is a boundary"
    );
    assert!(
        banks.pattern().lanes()[0].slot_on(5),
        "and the sixteen slots underneath are untouched — including the odd ones an eighth \
         does not read"
    );
    // A bank, and one this session does not have.
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &View::new(Room::Day),
        &Operation::SelectPattern { pattern: 3 }
    )
    .is_some());
    assert_eq!(banks.armed(), 3);
    assert!(
        banks.pattern().is_empty(),
        "bank 3 is one of the three empty ones"
    );
    let refused = sequenced(
        &mut banks,
        &mut playhead,
        &View::new(Room::Day),
        &Operation::SelectPattern { pattern: 9 },
    )
    .expect("a press this session cannot perform says so rather than going quiet");
    assert!(refused.contains("there is no bank 9"));
    assert_eq!(banks.armed(), 3, "and a refused press moves nothing");
}

/// A lane arrives with its two levels filled in from the range the console was
/// published, and it arrives muted.
///
/// This is the half of `+ lane` no test in `karakuri-console` can see: that bay
/// hands back `Operation::PointLane { pattern, target }` and appends nothing,
/// and the payload carries no levels — a fader's are 1.0 and 0.0 and a
/// parameter's are the range `View::inspector` holds, which is the same reading
/// the chooser drew its items from (ADR-0320, ADR-0327).
#[test]
fn a_pointed_lane_takes_its_levels_from_the_published_range() {
    let mut banks = karakuri_pattern::Banks::default();
    let mut playhead = karakuri_pattern::Playhead::default();
    let param = karakuri_operation::ParamAt {
        node: Some(karakuri_operation::NodeAddress {
            layer: karakuri_operation::Layer::L2,
            index: 0,
        }),
        key: "twist".to_owned(),
    };
    let mut view = View::new(Room::Day);
    view.inspector = vec![view::Pane {
        deck: 1,
        material: "lattice_veil".to_owned(),
        sync: karakuri_operation::Sync::Free,
        allows: [true; view::SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: false,
        // Not this test's row: the deck head's two build chips are
        // drawn from this and nothing here is about them.
        aimed: None,
        nodes: vec![view::Node {
            addr: "L2:0".to_owned(),
            name: "warp".to_owned(),
            authority: None,
            // Not this test's control either: nothing here presses a
            // node head's `keep`.
            keep: None,
            uses: Vec::new(),
            renderers: Vec::new(),
            params: vec![view::Param {
                ord: Some(1),
                name: "twist".to_owned(),
                value: 1.0,
                // **Not `[0, 1]`**, so a range that was read and a pair
                // that was assumed cannot look alike.
                range: [0.25, 4.0],
                param: param.clone(),
                bound: None,
            }],
        }],
    }];

    // A fader's pair is the gate, and no reading is consulted for it.
    let line = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::PointLane {
            pattern: 0,
            target: karakuri_operation::LaneTarget::Fader { deck: 0 },
        },
    )
    .expect("a lane press says what it did");
    assert!(line.contains("on 1 off 0"), "a fader's gate: {line}");
    let lane = &banks.pattern().lanes()[0];
    assert_eq!((lane.on(), lane.off()), (1.0, 0.0));
    assert!(
        lane.muted(),
        "a lane arrives with every slot off, and an off step writes `off` — so an unmuted \
         fader lane would hold its deck at zero from the press"
    );

    // A parameter's pair is the published range, top then bottom.
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::PointLane {
            pattern: 0,
            target: karakuri_operation::LaneTarget::Param {
                deck: 1,
                param: param.clone(),
            },
        }
    )
    .is_some());
    let lane = &banks.pattern().lanes()[1];
    assert_eq!(
        (lane.on(), lane.off()),
        (4.0, 0.25),
        "an on step writes the top of the published range and an off step the bottom"
    );

    // **A control this console holds no row for is refused and said**,
    // rather than defaulted into two numbers nobody chose.
    let refused = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::PointLane {
            pattern: 0,
            target: karakuri_operation::LaneTarget::Param {
                deck: 3,
                param: param.clone(),
            },
        },
    )
    .expect("a press this window cannot perform says so rather than going quiet");
    assert!(refused.contains("no published range"), "{refused}");
    assert_eq!(
        banks.pattern().lanes().len(),
        2,
        "and a refused press appends nothing"
    );
}

/// A lane is taken out by its position, the lanes after it move up, and an index
/// this pattern has not got is refused in `no_such_slot`'s own sentence.
///
/// The other half of the minus at the end of a lane's row: the bay hands back
/// `Operation::RemoveLane { pattern, lane }` and takes nothing out itself, so
/// this is where the pattern actually loses the row (ADR-0352's property read on
/// a lane).
#[test]
fn a_removed_lane_takes_its_steps_with_it_and_the_rest_move_up() {
    let mut banks = karakuri_pattern::Banks::default();
    let mut playhead = karakuri_pattern::Playhead::default();
    let view = View::new(Room::Day);
    for deck in 0..3 {
        assert!(sequenced(
            &mut banks,
            &mut playhead,
            &view,
            &Operation::PointLane {
                pattern: 0,
                target: karakuri_operation::LaneTarget::Fader { deck },
            }
        )
        .is_some());
    }
    assert_eq!(banks.pattern().lanes().len(), 3);

    let line = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::RemoveLane {
            pattern: 0,
            lane: 1,
        },
    )
    .expect("a lane press says what it did");
    assert!(line.contains("taken out"), "{line}");
    assert!(
        line.contains("no record"),
        "a pattern is not a session record: {line}"
    );
    assert_eq!(banks.pattern().lanes().len(), 2);
    assert_eq!(
        banks.pattern().lanes()[1].target(),
        &karakuri_operation::LaneTarget::Fader { deck: 2 },
        "the lanes after the removed one move up, which is what a lane index means"
    );

    // **An index this pattern has not got is refused in the words every
    // surface refuses an address in**: the thing named, then what there was to
    // name (`karakuri_environment::no_such_slot`).
    let refused = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::RemoveLane {
            pattern: 0,
            lane: 7,
        },
    )
    .expect("a press this window cannot perform says so rather than going quiet");
    assert!(
        refused.contains("no lane 7") && refused.contains("holds lanes 0-1"),
        "{refused}"
    );
    assert_eq!(
        banks.pattern().lanes().len(),
        2,
        "and a refused press takes nothing out"
    );

    // A bank this session does not hold is the other refusal, and it is the
    // one every arm of this handler makes.
    let refused = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::RemoveLane {
            pattern: 9,
            lane: 0,
        },
    )
    .expect("a press naming no bank says so");
    assert!(
        refused.contains("not a bank this session holds"),
        "{refused}"
    );

    // **The lane that is driving comes out too**, and nothing refuses it: a
    // lane's writes are its whole record, so they stop here (ADR-0322).
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::SetLaneMute {
            pattern: 0,
            lane: 0,
            muted: false,
        }
    )
    .is_some());
    assert_eq!(
        banks.pattern().held().next(),
        Some((0, &karakuri_operation::LaneTarget::Fader { deck: 0 })),
        "an unmuted lane holds the control it drives"
    );
    let line = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::RemoveLane {
            pattern: 0,
            lane: 0,
        },
    )
    .expect("a lane press says what it did");
    assert!(line.contains("It was driving"), "{line}");
    assert_eq!(
        banks.pattern().held().count(),
        0,
        "and nothing holds deck A's fader once the lane that did is gone"
    );

    // The last lane out leaves an empty pattern, and the refusal then says
    // there is nothing to name.
    assert!(sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::RemoveLane {
            pattern: 0,
            lane: 0,
        }
    )
    .is_some());
    assert!(banks.pattern().is_empty());
    let refused = sequenced(
        &mut banks,
        &mut playhead,
        &view,
        &Operation::RemoveLane {
            pattern: 0,
            lane: 0,
        },
    )
    .expect("a press on an empty pattern says so");
    assert!(refused.contains("this pattern holds none"), "{refused}");
}
