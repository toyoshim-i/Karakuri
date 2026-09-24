use super::*;

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

/// Verifies that SetMaskPosition preserves existing mask shape and angle from Current::mask.
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

/// Verifies that missing Current::mask returns Written::Owed rather than defaulting.
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

/// Verifies that SetSync anchors at session tempo and resets scrub offset without inheriting prior transport state.
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

/// Verifies that SetSync without session tempo in Current returns Written::Owed.
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
