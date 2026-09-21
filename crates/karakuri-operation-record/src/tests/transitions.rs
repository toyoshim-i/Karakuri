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
