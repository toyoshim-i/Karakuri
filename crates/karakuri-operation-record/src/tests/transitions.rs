use super::*;

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

/// Verifies that Crossfade produces 4 records (silencing and making live arriving deck, then fading both).
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

/// Verifies that SelectRenderer translates as an instant cut reading only the transition start instant.
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

/// Verifies that cuts retain the exact requested start beat and zero duration.
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

/// Verifies that Wipe emits 6 records (mask setup, opacity, blend, residency, and transition move).
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

/// Verifies that Wipe only writes records targeting the arriving deck, leaving the covered deck untouched.
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

/// Verifies that Wipe requires Current::mask for softness configuration, returning Written::Owed if missing.
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

/// Verifies that Wipe preserves pre-existing blend mode and live residency on arriving deck.
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

/// Verifies that a wipe on a deck already under over-layering emits put-on-air only.
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

/// Verifies that Wipe requires Current::mix and returns Written::Owed if unread.
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

/// Verifies that scheduled operations return Written::Owed(Reading::Transition) when transition settings are missing.
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
