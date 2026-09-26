use super::*;

/// Verifies transport controls convert to expected session records or return explicit unwritten/silent outcomes (ADR-0185, ADR-0194).
#[test]
fn a_controls_operation_becomes_the_record_the_cli_would_have_written() {
    assert_eq!(
        only_record(&Operation::SetGain {
            deck: 2,
            gain: 0.75
        }),
        // What `mix::gain_record(2, 0.75)` wrote, before ADR-0194 deleted
        // it in favour of this conversion.
        Record::Gain {
            slot: DeckSlot(2),
            value: 0.75
        }
    );
    assert_eq!(
        only_record(&Operation::SetOpacity {
            deck: 0,
            opacity: 0.25
        }),
        // `mix::opacity_record(0, 0.25)`.
        Record::Opacity {
            slot: DeckSlot(0),
            value: 0.25
        }
    );
    // Verify each blend mode in the cycle produces the corresponding wire record.
    for (deck, blend) in BlendMode::ALL.into_iter().enumerate() {
        let deck = deck as u8;
        assert_eq!(
            only_record(&Operation::SetBlendMode { deck, blend }),
            // `mix::blend_record(deck, blend)`.
            Record::Blend {
                slot: DeckSlot(deck),
                mode: blend.name().to_owned(),
            },
            "`{}` did not become the record `mix::blend_record` writes",
            blend.name()
        );
        // And the engine reads its own name back, which is what says the
        // two lists are the same three words rather than two spellings of
        // them.
        assert_eq!(
            Blend::from_name(blend.name()),
            Some(blend_mode_back(blend)),
            "the engine does not know the vocabulary's `{}`",
            blend.name()
        );
    }

    // Wire protocol residency names match across all cycle states (`mix::residency_wire_name`).
    for (deck, (residency, level)) in [
        (karakuri_operation::Residency::Live, "live"),
        (karakuri_operation::Residency::Priming, "priming"),
        (karakuri_operation::Residency::Allocated, "allocated"),
    ]
    .into_iter()
    .enumerate()
    {
        let deck = deck as u8;
        assert_eq!(
            only_record(&Operation::SetResidency { deck, residency }),
            // `mix::residency_record(deck, residency)`.
            Record::Residency {
                slot: DeckSlot(deck),
                level: level.to_owned(),
            },
            "{residency:?} did not become the record `mix::residency_record` writes"
        );
        // And it reads back as the level it named, which is what says the
        // two spellings are one list rather than two.
        assert_eq!(
            mix::parse_residency(level),
            Some(residency_back(residency)),
            "the wire spelling `{level}` does not come back as {residency:?}"
        );
    }

    // Verifies unmapped operations return typed non-record indications rather than ambiguous defaults.
    assert_eq!(
        written(&Operation::Solo { region: None }, &Current::default()),
        Written::Silent(Silent::Surface)
    );
    assert_eq!(
        written(&Operation::SelectDeck { deck: 1 }, &Current::default()),
        Written::Silent(Silent::Surface)
    );
}

/// Verifies that scrub operations (relative) and sync mode selections (absolute) convert into records correctly.
#[test]
fn the_deck_heads_two_operations_go_different_distances() {
    // Scrub operations write relative offsets combining current position with requested offset.
    let current = Current {
        transport: Some(karakuri_operation_record::Transport {
            sync: karakuri_operation::Sync::Beat,
            anchor_bpm: 128.0,
            scrub_beats: -1.5,
        }),
        ..Current::default()
    };
    // Verify scrub uses the console's scrub constant.
    let scrub = Operation::ScrubDeck {
        deck: 1,
        beats: SCRUB_BEATS,
    };
    assert_eq!(
        written(&scrub, &current),
        Written::Records(vec![Record::Transport {
            slot: DeckSlot(1),
            sync: "beat".to_owned(),
            anchor_bpm: 128.0,
            scrub_beats: -1.25,
        }]),
        "a press of the deck head's forward arrow, from -1.50, did not come out at -1.25 — \
         so the record is not the offset the deck holds plus the amount the arrow asks for"
    );
    // Missing transport state must result in an owed read rather than zero-init.
    assert_eq!(
        written(&scrub, &Current::default()),
        Written::Owed(Owed::NotRead(karakuri_operation_record::Reading::Transport)),
        "a scrub with no transport read came back with a record, which means it invented \
         the position it moved from"
    );
    // Engine round-trips sync mode wire names matching vocabulary.
    for sync in SYNCS {
        assert_eq!(
            EngineSync::from_name(sync.name()).map(mix::sync),
            Some(sync),
            "the engine does not know the vocabulary's `{}`",
            sync.name()
        );
    }

    // Sync mode engagement clears prior scrub offsets and anchors to current session tempo.
    let set = Operation::SetSync {
        deck: 1,
        sync: karakuri_operation::Sync::Beat,
    };
    let engaged = Current {
        tempo: Some(126.0),
        ..current
    };
    assert_eq!(
        written(&set, &engaged),
        Written::Records(vec![Record::Transport {
            slot: DeckSlot(1),
            sync: "beat".to_owned(),
            anchor_bpm: 126.0,
            scrub_beats: 0.0,
        }]),
        "a press of the deck head's sync chip, in a room at 126 bpm, did not come out \
         anchored at 126 with the scrub cleared — either the slot's old anchor survived \
         being re-engaged, or the position it was scrubbed to did"
    );
    // Missing tempo state must yield an owed read rather than guessing.
    assert_eq!(
        written(&set, &Current::default()),
        Written::Owed(Owed::NotRead(karakuri_operation_record::Reading::Tempo)),
        "a sync mode with no session tempo read came back with a record, which means the \
         tempo it anchored the deck at was invented"
    );
    let said = unwritten(&set, &written(&set, &Current::default())).expect(
        "a sync chip press with no tempo read said nothing at all — a press that reads, in \
         silence, exactly like a press that did not work",
    );
    assert!(
        said.contains("SetSync")
            && said.contains(Owed::NotRead(karakuri_operation_record::Reading::Tempo).why()),
        "the window said `{said}`, which does not name both the operation and the reading \
         it did not get"
    );
}

/// Verifies sync mode ordering remains consistent across console inspector layouts and engine arrays.
#[test]
fn the_two_crates_walk_the_sync_modes_in_one_order() {
    assert_eq!(EngineSync::ALL.len(), SYNCS.len());
    for (index, mode) in EngineSync::ALL.into_iter().enumerate() {
        assert_eq!(
            mix::sync(mode),
            SYNCS[index],
            "`EngineSync::ALL[{index}]` is `{}` and the console's `SYNCS[{index}]` is \
             `{}` — the deck head's cycle would skip the wrong mode",
            mode.name(),
            SYNCS[index].name()
        );
    }
}

/// Verifies the animation event loop requests new frames whenever any preview or picture sink remains active.
#[test]
fn anything_that_makes_texels_keeps_the_loop_awake() {
    let some = Picture {
        id: egui::TextureId::User(0),
        rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(16.0, 9.0)),
    };
    let mut view = View::new(Room::Day);

    // Nothing is making texels, so the loop has no reason of its own to
    // draw and `ControlFlow::Wait` gets to block.
    assert!(!live(&view), "an empty panel was called live");

    // The picture, which is what this rule used to be the whole of.
    view.picture = Some(some);
    assert!(live(&view), "a live picture did not keep the loop awake");

    // Verify preview rendering keeps the loop awake even when picture is folded.
    view.picture = None;
    view.previews[0] = Some(some);
    assert!(
        live(&view),
        "the picture is folded away and deck A is still rendering, and the loop was \
         told to sleep — which is the window that kept drawing 270 frames after it \
         was said to have gone quiet"
    );

    // And the list is closed: every cell counts, including the three this
    // program leaves off, because the bug is a sink that is not read here.
    for deck in 0..DECKS {
        let mut view = View::new(Room::Day);
        view.previews[deck] = Some(some);
        assert!(
            live(&view),
            "deck {deck} is rendering and the loop was told to sleep"
        );
    }

    // The other direction, which costs frames rather than pixels: with
    // every sink off the loop stops asking.
    view.previews[0] = None;
    assert!(
        !live(&view),
        "nothing is rendering and the loop stayed awake"
    );
}
