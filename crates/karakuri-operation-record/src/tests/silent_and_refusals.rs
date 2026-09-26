use super::*;

/// Verifies that arrangement lifecycle operations write surface files and emit Written::Silent(Surface) (ADR-0221).
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

/// Verifies that pattern configuration operations write library files and emit Written::Silent(Surface) (ADR-0227, ADR-0320).
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
        Operation::RemoveLane {
            pattern: 0,
            lane: 0,
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

/// Verifies that query operations (e.g. WalkHistory) emit Written::Silent(Question) (ADR-0342).
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

/// The lanes reading with one lane holding `deck`'s fader, and a lane in
/// front of it that holds something else — so a refusal naming the first
/// lane it found rather than the lane that holds the fader is visible here
/// (ADR-0323).

#[test]
fn a_scheduled_move_on_a_held_control_is_refused_and_writes_no_record() {
    let current = Current {
        transition: Some(transition()),
        mask: Some(mask()),
        mix: Some(mix()),
        lanes: Some(holding(1)),
        ..Current::default()
    };
    // Every scheduling operation that can meet a lane: the fade on the deck
    // it names, the crossfade on either end, and the wipe on the deck
    // arriving — whose fader it writes to full for the mask to reveal.
    for operation in [
        Operation::FadeDeck { deck: 1, to: 0.0 },
        Operation::Crossfade { from: 1, to: 2 },
        Operation::Crossfade { from: 0, to: 1 },
        Operation::Wipe { from: 0, to: 1 },
    ] {
        assert_eq!(
            written(&operation, &current),
            Written::Refused(Refusal { lane: 1, deck: 1 }),
            "`{operation:?}` over a lane holding deck 1's fader was not refused, or \
                 was refused naming another lane — the lane in front of it holds a \
                 parameter on another deck"
        );
    }
    assert_eq!(
        Refusal { lane: 1, deck: 1 }.why(),
        "deck 1's fader is held by lane 1 of the armed pattern: mute that lane and \
             ask again",
        "the one sentence changed wording — every surface says this one, and the \
             next attempt it carries is the mute"
    );
}

/// Verifies that muted lanes or unread lane states do not trigger move refusals (ADR-0323).
#[test]
fn a_muted_lane_and_a_reading_nobody_took_refuse_nothing() {
    let over = Current {
        transition: Some(transition()),
        mask: Some(mask()),
        mix: Some(mix()),
        ..Current::default()
    };
    for lanes in [None, Some(Lanes::default()), Some(holding(3))] {
        let current = Current {
            lanes: lanes.clone(),
            ..over.clone()
        };
        for operation in [
            Operation::FadeDeck { deck: 1, to: 0.0 },
            Operation::Crossfade { from: 0, to: 1 },
            Operation::Wipe { from: 0, to: 1 },
        ] {
            assert_eq!(
                written(&operation, &current),
                written(&operation, &over),
                "`{operation:?}` against `{lanes:?}` wrote something other than what \
                     it writes with no lane holding anything — a lane that was muted, \
                     or a reading nobody took, refuses nothing"
            );
            assert!(
                matches!(written(&operation, &current), Written::Records(_)),
                "`{operation:?}` wrote no records at all, so this half is asserting \
                     two refusals are equal rather than that the move still schedules"
            );
        }
    }
}

/// Verifies that SelectRenderer is unaffected by lanes holding channel faders (ADR-0323).
#[test]
fn a_renderer_choice_is_untouched_by_a_lane_on_the_same_deck() {
    let current = Current {
        transition: Some(transition()),
        lanes: Some(holding(1)),
        ..Current::default()
    };
    assert_eq!(
        records(written(
            &Operation::SelectRenderer {
                deck: 1,
                renderer: 2
            },
            &current
        )),
        vec![Record::Select {
            slot: DeckSlot(1),
            renderer: 2,
            start: 37.0,
        }],
        "a renderer choice on a deck whose fader a lane holds was refused or \
             rewritten — a selection moves no control a lane can drive"
    );
}
