use super::*;

/// A control's operation becomes the record every other surface's control ends
/// in, and this is the half of that which needs no device.
///
/// This test is older than the conversion it now checks, and that is the point
/// of it. It was written against the hand-written `record` this file used to
/// carry, asserting term for term what `karakuri-cli`'s `mix::gain_record` and
/// `mix::opacity_record` already wrote. That function is deleted and
/// [`written`] answers instead (ADR-0185's promise, kept where ADR-0194 put the
/// home) — every expectation below is unchanged, so if the crate's conversion
/// disagreed with the one that was deleted, this is what says so.
///
/// `mix::gain_record` is deleted too, by the same record and for the stronger
/// reason: the conversion *is* the derivation now, and two of them is the drift
/// `mix.rs` exists to end. The comments below name it where it stood, because
/// what this test compares against is the record that function wrote rather
/// than the function.
///
/// And the other direction: an operation this program has no control for writes
/// no record here either, and the answer says *which* kind of nothing rather
/// than a bare `None` — which is the whole of what the three answers buy.
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
    // **Every mode of the cycle, because a chip that emits three
    // operations has three records to write** — and the mode is a wire
    // name, so a mode that reached `Record::Blend` misspelled would be
    // refused by the engine on the way back rather than here.
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

    // **Every residency of the cycle**, for the same reason as the blend:
    // one chip emitting three operations has three records to write. The
    // spelling is the wire's — `mix::residency_wire_name`'s three words,
    // which are deliberately not the status line's `LIVE`/`prim`/`park`
    // and not the chip's `live`/`prim`/`alloc` either, so a record written
    // in the chip's vocabulary would decode as nothing at all.
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

    // The vocabulary is larger than what this program reaches: five controls
    // writing five records. A record invented for the other 45 would be
    // somebody deciding what they mean — and the answer is now *which*
    // nothing rather than `None`, because a surface's own state and a
    // record nobody can write yet are not the same silence.
    assert_eq!(
        written(&Operation::Solo { region: None }, &Current::default()),
        Written::Silent(Silent::Surface)
    );
    assert_eq!(
        written(&Operation::SelectDeck { deck: 1 }, &Current::default()),
        Written::Silent(Silent::Surface)
    );
}

/// The deck head's two operations, as far as this program can take them without
/// a device — and they go the same distance now, which is the point.
///
/// They used to go different distances: a scrub became a record and a sync mode
/// did not, and the second half of that is what `tests/panel_column.rs`'s one
/// exemption rested on — the chip's badge stayed `plan` because an operator who
/// pressed it reached the emission and not the move. That test said the day it
/// stopped being true it would stop being true here, and this is here.
///
/// The two are still not the same conversion, and that is what the second half
/// asserts. A scrub is relative and reads the transport it moves from; a mode
/// is absolute and reads the session tempo, replacing the anchor and clearing
/// the scrub. A sync mode that came out carrying the position the slot was
/// scrubbed to would be the two conversions having been made one.
#[test]
fn the_deck_heads_two_operations_go_different_distances() {
    // **The scrub is relative, so the record is where the slot is plus
    // what was asked for.** The reading is handed in by hand here for
    // `reading`'s reason at the mask: there is no deck in this test
    // binary, and what is being checked is the arithmetic rather than the
    // read.
    let current = Current {
        transport: Some(karakuri_operation_record::Transport {
            sync: karakuri_operation::Sync::Beat,
            anchor_bpm: 128.0,
            scrub_beats: -1.5,
        }),
        ..Current::default()
    };
    // **The amount is the console's own constant**, not a figure written
    // again here: the arrow that emits it and the record that carries it
    // are one number or the panel and the deck disagree about how far a
    // press goes.
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
    // **And the reading is what makes it one**: without it the conversion
    // says so rather than starting the deck's scrub from zero, which is
    // why `reading` has an arm for this operation at all.
    assert_eq!(
        written(&scrub, &Current::default()),
        Written::Owed(Owed::NotRead(karakuri_operation_record::Reading::Transport)),
        "a scrub with no transport read came back with a record, which means it invented \
         the position it moved from"
    );
    // **The mode goes out as a wire name and the engine reads its own name
    // back**, which is what `apply` does with it and is the blend chip's
    // assertion one control along.
    for sync in SYNCS {
        assert_eq!(
            EngineSync::from_name(sync.name()).map(mix::sync),
            Some(sync),
            "the engine does not know the vocabulary's `{}`",
            sync.name()
        );
    }

    // **A sync mode anchors at the session tempo and starts on the
    // grid.** The reading handed in is the same one the scrub used —
    // anchored at 128 and scrubbed to -1.5 — and none of it may survive:
    // `Transport::engaged` clears the scrub because *"a slot brought back
    // to the grid should be on the grid, not on wherever it was scrubbed
    // to a song ago"*, and the anchor is the room's tempo rather than the
    // one the slot was last locked to.
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
    // **And the reading is what makes it one.** Without the tempo the
    // conversion says so rather than anchoring at a guess, which is the
    // scrub's own arrangement two assertions up and the reason `reading`
    // has an arm for this operation at all.
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

/// The two crates walk the sync modes in one order, which is what makes
/// `view::Pane::allows` line up with the field it fills.
///
/// [`inspector`] builds that array by mapping `EngineSync::ALL` and the console
/// reads it by indexing [`SYNCS`], so the two orders are one order or the panel
/// skips the wrong mode — silently, and only on material that refuses
/// something. Two arrays cannot be made one by a comment.
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

/// Anything that makes texels this frame keeps the loop awake, and the list is
/// closed.
///
/// [`live`] decides whether the loop asks for another frame, and it is the one
/// decision in this file that has already been got wrong twice in the same
/// direction. The first time it was set once and never cleared, so folding the
/// picture away left the window drawing at full rate — found by an operator on
/// another machine following this file's own instructions, which said the
/// window goes quiet, and getting 270 frames. The second time it was the
/// picture alone, which is the same failure with a preview under it: fold the
/// picture and deck A goes on auditioning while the loop stops asking for
/// frames, so the panel keeps changing and nothing draws it.
///
/// So the assertion is over every sink, not over the one this program fills: a
/// cell nobody has wired up yet is asserted live all the same, because the
/// failure is a sink left out of the list rather than a sink that is off.
///
/// It needs no device: an `egui::TextureId` is a number, and what is being
/// asserted is a rule about `Option`s.
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

    // **The case the picture-alone rule gets wrong**: the picture folded
    // away with deck A still auditioning under it.
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
