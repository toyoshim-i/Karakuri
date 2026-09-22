use super::deck_head_common::*;

// ---------------------------------------------------------------------------
// What a press asks for
// ---------------------------------------------------------------------------

/// Verifies that the sync chip cycles through all sync modes and emits `SetSync`.
#[test]
fn the_chip_cycles_every_mode_once_and_wraps() {
    let (panel, ctx) = console(PLAUSIBLE);
    let mut seen = Vec::new();
    let mut at_now = Sync::Free;
    for _ in 0..EVERY.len() {
        let pane = at_sync(at_now);
        let (_, head) = chips(&panel, &ctx, 0, &pane);
        let asked = head
            .sync(at(head.mode.center()))
            .expect("a press on the sync chip");
        let Operation::SetSync { deck, sync } = asked else {
            panic!("a press on the sync chip asked for {asked:?}")
        };
        assert_eq!(deck, pane.deck as u8, "the press named another deck");
        assert_ne!(
            sync, at_now,
            "the chip asked for the mode that is already running, and nothing this deck's \
             material refuses"
        );
        seen.push(sync);
        at_now = sync;
    }
    assert_eq!(
        at_now,
        Sync::Free,
        "three presses from `free` did not come back to it, so the cycle is not three long"
    );
    for sync in EVERY {
        assert_eq!(
            seen.iter().filter(|seen| **seen == sync).count(),
            1,
            "`{}` is reached {} times in one turn of the cycle",
            sync.name(),
            seen.iter().filter(|seen| **seen == sync).count()
        );
    }
}

/// The cycle skips a mode this material cannot honour rather than offering it,
/// which is the one thing this cycle has that the mixer's two do not — and the
/// reading it skips on is the engine's, handed in.
#[test]
fn the_cycle_skips_a_mode_the_material_cannot_honour() {
    let (panel, ctx) = console(PLAUSIBLE);
    let asked = |pane: &Pane| {
        let (_, head) = chips(&panel, &ctx, 0, pane);
        match head.sync(at(head.mode.center())) {
            Some(Operation::SetSync { sync, .. }) => sync,
            other => panic!("a press on the sync chip asked for {other:?}"),
        }
    };

    // Material that accumulates: `beat` is refused, so `tempo` steps past it
    // to `free` rather than offering a mode the engine would turn down.
    let accumulates = Pane {
        allows: [true, true, false],
        ..at_sync(Sync::Tempo)
    };
    assert_eq!(asked(&accumulates), Sync::Free);
    assert_eq!(
        asked(&Pane {
            sync: Sync::Free,
            ..accumulates.clone()
        }),
        Sync::Tempo
    );

    // Material that accumulates **and** reads the beat: only `free` is left,
    // so the one mode a cycle can reach is the mode it is in — which
    // re-anchors, and is the one case where re-anchoring does nothing.
    let both = Pane {
        allows: [true, false, false],
        ..at_sync(Sync::Free)
    };
    assert_eq!(
        asked(&both),
        Sync::Free,
        "a cycle over one available mode landed somewhere else, so it offered a mode the deck \
         would refuse"
    );
}

/// The anchor asks for the mode the deck is already in, which re-anchors — and
/// it is the one thing the chip beside it can never say, because a cycle starts
/// at the mode *after* the one you are on.
#[test]
fn the_anchor_asks_for_the_mode_the_deck_is_already_in() {
    let (panel, ctx) = console(PLAUSIBLE);
    for sync in [Sync::Tempo, Sync::Beat] {
        let pane = at_sync(sync);
        let (_, head) = chips(&panel, &ctx, 0, &pane);
        let anchor = head.anchor.expect("a synced deck reads an anchor");
        assert_eq!(
            head.reanchor(at(anchor.center())),
            Some(Operation::SetSync {
                deck: pane.deck as u8,
                sync
            }),
            "the anchor asked for something other than the mode the deck is in"
        );
        // And the chip cannot: with nothing refused it always steps away.
        let stepped = match head.sync(at(head.mode.center())) {
            Some(Operation::SetSync { sync, .. }) => sync,
            other => panic!("a press on the sync chip asked for {other:?}"),
        };
        assert_ne!(
            stepped, sync,
            "the cycle reached the mode the deck is in, so the anchor is not the only control \
             that can re-anchor and this file's reason for it is wrong"
        );
    }
}

/// A free deck has no anchor and nothing to re-ask for. Free reads no anchor at
/// all, so there is nothing a press could re-anchor to.
#[test]
fn a_free_deck_has_nothing_to_re_anchor() {
    let pane = at_sync(Sync::Free);
    let (panel, ctx) = console(PLAUSIBLE);
    let (pane_at, head) = chips(&panel, &ctx, 0, &pane);
    assert_eq!(head.anchor, None);
    for probe in [
        head.mode.right_top(),
        egui::pos2(head.mode.max.x + 1.0, head.mode.center().y),
        pane_at.deck_head.center(),
    ] {
        assert_eq!(head.reanchor(at(probe)), None);
    }
}

/// An arrow asks for a quarter beat, and which arrow it was is the sign — the
/// one control on this panel that moves by an amount, because there is no
/// destination in the vocabulary for it to name.
#[test]
fn the_arrows_ask_for_a_quarter_beat_each_way() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let (_, head) = chips(&panel, &ctx, 0, &pane);
    assert_eq!(
        head.scrub(at(head.back.center())),
        Some(Operation::ScrubDeck {
            deck: pane.deck as u8,
            beats: -SCRUB_BEATS
        })
    );
    assert_eq!(
        head.scrub(at(head.forward.center())),
        Some(Operation::ScrubDeck {
            deck: pane.deck as u8,
            beats: SCRUB_BEATS
        })
    );
    assert_eq!(
        SCRUB_BEATS, 0.25,
        "the row this fills is titled *Scrub a deck a quarter beat*"
    );

    // **The amount does not depend on where the offset already is**, which is
    // what *relative* means and is why the record needs a reading.
    let far = Pane {
        scrub_beats: -12.75,
        ..pane.clone()
    };
    let (_, moved) = chips(&panel, &ctx, 0, &far);
    assert_eq!(
        moved.scrub(at(moved.forward.center())),
        Some(Operation::ScrubDeck {
            deck: far.deck as u8,
            beats: SCRUB_BEATS
        })
    );
}

/// A press is one of the six or none, so no point on the row asks two
/// questions.
#[test]
fn a_press_is_one_control_or_none() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let (pane_at, head) = chips(&panel, &ctx, 0, &pane);
    let anchor = head.anchor.expect("an anchor");
    let aim = head.aim.expect("a wide pane draws the two build chips");
    for probe in [
        head.mode.center(),
        anchor.center(),
        head.back.center(),
        head.forward.center(),
        aim.size.center(),
        aim.salt.center(),
        head.composite.center(),
        pane_at.deck_head.left_center(),
        pane_at.deck_head.right_center(),
    ] {
        let asked = [
            head.sync(at(probe)),
            head.reanchor(at(probe)),
            head.scrub(at(probe)),
            head.resized(at(probe)),
            head.re_salted(at(probe)),
            head.compositing(at(probe)),
        ];
        let answered = asked.iter().filter(|a| a.is_some()).count();
        assert!(
            answered <= 1,
            "a press at {probe:?} asked {answered} of the six controls for something"
        );
        assert_eq!(
            head.owns(at(probe)),
            answered == 1,
            "`owns` and what a press asks for disagree at {probe:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// The route a window loop takes
// ---------------------------------------------------------------------------

/// The fold names the layering the deck is not in, and never a step.
///
/// Two panes, one compositing and one overdrawing, and the same chip on each:
/// what leaves is `SetCompositing` carrying the destination, computed from the
/// state the frame that laid the row out drew — which is
/// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// and `Mixer::blend`'s division. Nothing in the vocabulary says *toggle*, and
/// a control that could only step would leave two surfaces disagreeing about
/// where the deck is.
///
/// It does not claim reachability: whether the press re-aims the slot is
/// `crates/karakuri/src/main.rs`'s, which this crate cannot depend on
/// (ADR-0156). ADR-0314 is the record.
#[test]
fn the_fold_asks_for_the_layering_the_deck_is_not_in() {
    for composite in [false, true] {
        let pane = Pane {
            composite,
            ..mock()
        };
        let (mut panel, ctx) = console(PLAUSIBLE);
        let view = view(&pane);
        let (_, head) = chips(&panel, &ctx, 0, &pane);
        let probe = head.composite.center();

        assert_eq!(
            claim(&mut panel, &ctx, &view, at(probe)),
            Claim::Panel,
            "the fold is drawn and not claimed on a deck that composites: {composite}"
        );
        assert_eq!(
            head.compositing(at(probe)),
            Some(Operation::SetCompositing {
                deck: pane.deck as u8,
                compositing: !composite,
            }),
            "a press on the fold of a deck compositing {composite} did not ask for the other \
             layering"
        );
        // The other three answer nothing for it, so the destination cannot be
        // reached twice by one press — `a_press_is_one_control_or_none` is the
        // same fact over the whole row.
        assert_eq!(head.sync(at(probe)), None);
        assert_eq!(head.reanchor(at(probe)), None);
        assert_eq!(head.scrub(at(probe)), None);
    }
}

/// The capacity chip steps the powers of two inside the declared range, once
/// each, and wraps through the bottom — and what leaves is the number it
/// arrived at rather than a step, which is what a second surface would need to
/// agree with it (P-0090).
///
/// The whole ladder is walked rather than one press asserted, because a cycle
/// that had lost a rung, repeated one or stopped at the top would all pass a
/// single-press test.
#[test]
fn the_capacity_chip_steps_every_rung_once_and_wraps() {
    let (panel, ctx) = console(PLAUSIBLE);
    let mut at_capacity = LADDER[0];
    let mut visited = Vec::new();
    for _ in 0..LADDER.len() {
        let pane = Pane {
            aimed: Some(Aimed {
                capacity: at_capacity,
                ..aimed()
            }),
            ..mock()
        };
        let (_, head) = chips(&panel, &ctx, 0, &pane);
        let aim = head.aim.expect("a wide pane draws the two build chips");
        let asked = head.resized(at(aim.size.center()));
        let Some(Operation::SetProperty {
            deck,
            property: karakuri_operation::Property::Capacity { elements },
        }) = asked
        else {
            panic!("a press on the capacity chip at {at_capacity} asked for {asked:?}");
        };
        assert_eq!(deck, pane.deck as u8);
        visited.push(elements);
        at_capacity = elements;
    }
    let mut once = visited.clone();
    once.sort_unstable();
    once.dedup();
    assert_eq!(
        once,
        LADDER.to_vec(),
        "walking the ladder from its bottom visited {visited:?}, which is not every rung once"
    );
    assert_eq!(
        at_capacity, LADDER[0],
        "a full walk did not come back to the bottom, so the wrap goes somewhere else"
    );
}

/// A slot running at a number that is not on the ladder steps *up*, not back to
/// the bottom. `examples/beat_strands.kir` declares `capacity [4096, 1048576] =
/// 81920`, and a Set file may record anything the range allows, so this is an
/// ordinary state rather than a corner: a press that read as *one step* and
/// dropped the slot from 81920 to 4096 would be a control that reallocated
/// every element buffer in the wrong direction.
#[test]
fn a_capacity_off_the_ladder_steps_up() {
    let (panel, ctx) = console(PLAUSIBLE);
    let pane = Pane {
        aimed: Some(Aimed {
            capacity: 81_920,
            stated: true,
            capacities: LADDER.to_vec(),
            salt: NEXT_SALT,
        }),
        ..mock()
    };
    let (_, head) = chips(&panel, &ctx, 0, &pane);
    let aim = head.aim.expect("a wide pane draws the two build chips");
    assert_eq!(
        head.resized(at(aim.size.center())),
        Some(Operation::SetProperty {
            deck: pane.deck as u8,
            property: karakuri_operation::Property::Capacity { elements: 131_072 },
        })
    );
}

/// A chip with nowhere to step is drawn and claims nothing, which is the inert
/// scrub's arrangement one control along: two geometries whose declared ranges
/// do not overlap have no capacity one re-aim could send, and the number the
/// slot is running at is still worth reading.
#[test]
fn a_capacity_with_no_shared_range_is_drawn_and_claims_nothing() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let pane = Pane {
        aimed: Some(Aimed {
            capacities: Vec::new(),
            ..aimed()
        }),
        ..mock()
    };
    let view = view(&pane);
    let (_, head) = chips(&panel, &ctx, 0, &pane);
    let aim = head.aim.expect("the number is still drawn");
    assert!(
        aim.size.width() > 0.0,
        "a chip with nothing to step to lost its shape as well as its press"
    );
    assert_eq!(aim.resize, None);
    assert_eq!(head.resized(at(aim.size.center())), None);
    assert!(!head.owns(at(aim.size.center())));
    assert_eq!(
        claim(&mut panel, &ctx, &view, at(aim.size.center())),
        Claim::Egui,
        "a chip that asks for nothing is being claimed as a control the panel acts on"
    );
}

/// The `re-salt` capsule asks for the salt it was handed, and that is the whole
/// assertion: a console that derived one would be producing a picture a later
/// run could reproduce only by accident
/// ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
/// The number here is arbitrary on purpose — nothing in this crate can compute
/// it, so nothing in this crate can agree with a computation by luck.
#[test]
fn the_re_salt_capsule_asks_for_the_salt_it_was_handed() {
    let (panel, ctx) = console(PLAUSIBLE);
    let pane = mock();
    let (_, head) = chips(&panel, &ctx, 0, &pane);
    let aim = head.aim.expect("a wide pane draws the two build chips");
    assert_eq!(
        head.re_salted(at(aim.salt.center())),
        Some(Operation::SetProperty {
            deck: pane.deck as u8,
            property: karakuri_operation::Property::Seed { salt: NEXT_SALT },
        })
    );
    // The other five answer nothing for it, which is the same fact
    // `a_press_is_one_control_or_none` states over the whole row.
    assert_eq!(head.resized(at(aim.salt.center())), None);
    assert_eq!(head.compositing(at(aim.salt.center())), None);
}

/// The whole route, as the window loop drives it: `claim` first, then the pane
/// and the head derived a second time, then the operation.
///
/// It does not claim reachability. Whether a claimed press becomes one of these
/// operations is `crates/karakuri/src/main.rs`'s, which this crate cannot
/// depend on (ADR-0156).
#[test]
fn a_press_reaches_both_operations_the_way_the_window_loop_reaches_them() {
    let pane = mock();
    let (mut panel, ctx) = console(PLAUSIBLE);
    let view = view(&pane);
    let (_, laid) = chips(&panel, &ctx, 0, &pane);
    let anchor = laid.anchor.expect("an anchor");

    for (probe, expected) in [
        (
            laid.mode.center(),
            Operation::SetSync {
                deck: pane.deck as u8,
                sync: Sync::Free,
            },
        ),
        (
            anchor.center(),
            Operation::SetSync {
                deck: pane.deck as u8,
                sync: Sync::Beat,
            },
        ),
        (
            laid.back.center(),
            Operation::ScrubDeck {
                deck: pane.deck as u8,
                beats: -SCRUB_BEATS,
            },
        ),
        (
            laid.forward.center(),
            Operation::ScrubDeck {
                deck: pane.deck as u8,
                beats: SCRUB_BEATS,
            },
        ),
        (
            laid.composite.center(),
            Operation::SetCompositing {
                deck: pane.deck as u8,
                compositing: !pane.composite,
            },
        ),
    ] {
        assert_eq!(
            claim(&mut panel, &ctx, &view, at(probe)),
            Claim::Panel,
            "`claim` gives a press at {probe:?} to `egui`, so no route into the operation \
             exists however the control is drawn"
        );
        let again = inspector(panel.layout(), 0, &pane, 0.0)
            .and_then(|at| deck_head(&ctx, &at, &pane))
            .expect("the same head `claim` hit-tested");
        let asked = again
            .sync(at(probe))
            .or_else(|| again.reanchor(at(probe)))
            .or_else(|| again.scrub(at(probe)))
            .or_else(|| again.compositing(at(probe)))
            .expect("a press the panel claimed on a control it draws asks for something");
        assert_eq!(
            asked, expected,
            "the press was claimed and asked for something else, so the control that claims a \
             press and the control that acts on it have come apart"
        );
    }
}
