use super::sequencer_common::*;

// ---------------------------------------------------------------------------
// The bay head's bank pills
// ---------------------------------------------------------------------------

/// Verifies the four fixed bank pills in the bay head and their layout bounds (ADR-0320, ADR-0327).
#[test]
fn the_head_draws_four_bank_pills_and_they_are_inside_it() {
    let ctx = drawn_once();
    let panel = arranged(PLAUSIBLE, (1920, 1080));
    let view = view(StepMode::Sixteenth, Some(0));
    let bay = sequencer(
        &ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
    .unwrap();
    let region = rect_of(panel.layout(), "sequencer");
    assert_eq!(bay.banks.len(), BANKS, "one capsule per fixed bank");
    for (bank, pill) in bay.banks.iter().enumerate() {
        assert!(
            pill.min.y >= region.y && pill.max.y <= region.y + size::HEAD_H,
            "bank {bank}'s pill is in the bay head and not in the body"
        );
        assert!(
            pill.max.x <= region.x + region.w,
            "and inside the bay's own right edge"
        );
    }
    for pair in bay.banks.windows(2) {
        assert!(
            pair[0].max.x <= pair[1].min.x,
            "`seq 1` is left of `seq 2`, which is the order the mock draws them in"
        );
    }
    // And the mode pill in the body below is a different control, so a press
    // cannot mean both.
    assert!(
        bay.mode_pill.min.y >= region.y + size::HEAD_H,
        "the head's four and the body's one do not overlap"
    );
}

/// Verifies that pressing a bank pill explicitly selects that bank index rather than cycling.
#[test]
fn a_bank_press_names_the_bank_it_landed_on() {
    let ctx = drawn_once();
    let mut panel = arranged(PLAUSIBLE, (1920, 1080));
    let view = view(StepMode::Sixteenth, Some(0));
    let bay = sequencer(
        &ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
    .unwrap();
    for (bank, pill) in bay.banks.iter().enumerate() {
        let centre = pill.center();
        let at = Point {
            x: centre.x,
            y: centre.y,
        };
        assert_eq!(
            claim(&mut panel, &ctx, &view, at),
            Claim::Panel,
            "a bank pill is a control, so the press is the panel's"
        );
        match bay
            .press(at)
            .expect("a press on a bank pill asks for a bank")
        {
            Operation::SelectPattern { pattern } => assert_eq!(
                usize::from(pattern),
                bank,
                "the pill that was pressed and the bank in the payload are one answer"
            ),
            other => panic!("a press on a bank pill asked for {other:?}"),
        }
    }
    // **Including the armed one**, which is what makes it a state rather than
    // a move: asking for the bank you are on is allowed and does nothing.
    let armed = bay.banks[0].center();
    assert!(matches!(
        bay.press(Point {
            x: armed.x,
            y: armed.y
        }),
        Some(Operation::SelectPattern { pattern: 0 })
    ));
}

// ---------------------------------------------------------------------------
// The foot's `+ lane` and its chooser
// ---------------------------------------------------------------------------

/// A view with a pattern, four strips and a pane on deck A publishing two
/// controls — which is what the chooser has to have something to list.
fn choosing() -> View {
    let mut view = view(StepMode::Sixteenth, Some(0));
    view.mixer = std::iter::repeat_with(strip).take(4).collect();
    view.inspector = vec![karakuri_console::view::Pane {
        deck: 0,
        material: "drift_night".to_owned(),
        sync: karakuri_operation::Sync::Free,
        allows: [true; karakuri_console::view::SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: false,
        // Not this test's row: the deck head's two build chips are
        // drawn from this and nothing here is about them.
        aimed: None,
        nodes: vec![karakuri_console::view::Node {
            addr: "L2:0".to_owned(),
            name: "warp".to_owned(),
            authority: None,
            // Not this bay's control: nothing here presses a node head.
            keep: None,
            uses: Vec::new(),
            renderers: Vec::new(),
            params: vec![param("twist", 0), param("bend", 1)],
        }],
    }];
    view
}

/// A strip with nothing said about it — the mixer's length is all this file
/// reads off one.
fn strip() -> karakuri_console::view::Strip {
    karakuri_console::view::Strip {
        name: String::new(),
        tally: karakuri_console::view::Tally::Allocated,
        requested: karakuri_console::view::Tally::Allocated,
        gain: 0.0,
        gain_to: None,
        opacity: 0.0,
        opacity_to: None,
        blend: karakuri_operation::BlendMode::Over,
        mask: karakuri_console::view::Mask::None,
        mask_angle: 0.0,
        level: None,
        is_muted: false,
        is_soloed: false,
    }
}

/// One published control, over a range that is not `[0, 1]` — so a lane's two
/// levels can be told from a fader's.
fn param(name: &str, at: usize) -> karakuri_console::view::Param {
    karakuri_console::view::Param {
        ord: Some(at + 1),
        name: name.to_owned(),
        value: 0.0,
        range: [0.25, 4.0],
        param: karakuri_operation::ParamAt {
            node: Some(karakuri_operation::NodeAddress {
                layer: karakuri_operation::Layer::L2,
                index: 0,
            }),
            key: name.to_owned(),
        },
        // **Nothing bound**, which is what this file is about: a lane is a
        // fifth route and not a binding (ADR-0222).
        bound: None,
    }
}

/// The chooser lists every drawn deck's fader and one deck's published
/// controls, and the deck is the Library bay's load pulldown's (ADR-0305,
/// ADR-0327).
#[test]
fn the_chooser_lists_the_drawn_decks_faders_and_the_target_decks_keys() {
    let view = choosing();
    let choices = view.lane_choices();
    assert_eq!(choices.faders, 4, "one fader per strip the mixer draws");
    assert_eq!(
        choices.items.len(),
        6,
        "four faders and deck A's two published controls"
    );
    for (at, deck) in (0..4).enumerate() {
        assert_eq!(
            choices.items[at].target,
            LaneTarget::Fader { deck },
            "the faders are the decks in order"
        );
    }
    assert!(
        choices.items[0].words.starts_with('A'),
        "an item reads as the lane row it will make: {}",
        choices.items[0].words
    );
    for item in &choices.items[4..] {
        match &item.target {
            LaneTarget::Param { deck, param } => {
                assert_eq!(*deck, 0, "the parameters are the target deck's");
                assert!(
                    item.words.contains("L2:0") && item.words.contains(&param.key),
                    "an item names the node and the published control: {}",
                    item.words
                );
            }
            other => panic!("a parameter item points at {other:?}"),
        }
    }
    // **A deck the mixer draws no strip for is not offered**, which is
    // `View::select`'s refusal read again rather than a second rule.
    let mut three = choosing();
    three.mixer.truncate(3);
    assert_eq!(three.lane_choices().faders, 3);
    // **And a target deck the console holds no pane for offers no
    // parameters** — the reading's own limit, said rather than papered over.
    let mut elsewhere = choosing();
    assert!(elsewhere.aim_at(2));
    let choices = elsewhere.lane_choices();
    assert_eq!(
        choices.items.len(),
        choices.faders,
        "deck C has no pane, so the list is its faders alone"
    );
}

/// A press on `+ lane` puts the card down; a pick points the lane and names the
/// bank; every other press dismisses it.
#[test]
fn the_card_offers_the_targets_and_a_pick_points_the_lane() {
    let ctx = drawn_once();
    let mut view = choosing();
    let panel = arranged(PLAUSIBLE, (1920, 1080));
    let bay = sequencer(
        &ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
    .unwrap();
    assert!(bay.card.is_none(), "the card is up until somebody opens it");
    let add = bay.add.center();
    let on_pill = Point { x: add.x, y: add.y };
    assert!(
        matches!(
            bay.chose(on_pill, &view.lane_choices()),
            Some(karakuri_console::view::Chose::Open)
        ),
        "a press on the pill asks for the card"
    );
    assert!(view.open_lane(), "and the console puts it down");

    let choices = view.lane_choices();
    let bay = sequencer(&ctx, panel.layout(), view.sequencer.as_ref(), &choices).unwrap();
    let card = bay.card.expect("the card is down");
    assert_eq!(card.items, choices.items.len());
    assert!(
        card.ruled,
        "the faders and the parameters are two kinds, so the rule between them is drawn"
    );
    assert!(
        card.card.max.y <= bay.add.min.y,
        "it stands on the pill it hangs off, which is the foot's own edge"
    );
    // A pick of deck B's fader, which is the second item.
    let item = card.item(1).center();
    match bay
        .chose(
            Point {
                x: item.x,
                y: item.y,
            },
            &choices,
        )
        .expect("a press on an item picks it")
    {
        karakuri_console::view::Chose::Point(Operation::PointLane { pattern, target }) => {
            assert_eq!(pattern, 0, "the lane lands in the bank the bay was drawing");
            assert_eq!(target, LaneTarget::Fader { deck: 1 });
        }
        other => panic!("a pick asked for {other:?}"),
    }
    // A pick of the first parameter, which is after the rule.
    let item = card.item(4).center();
    match bay
        .chose(
            Point {
                x: item.x,
                y: item.y,
            },
            &choices,
        )
        .expect("a press on an item picks it")
    {
        karakuri_console::view::Chose::Point(Operation::PointLane { target, .. }) => {
            assert_eq!(
                target, choices.items[4].target,
                "the item that was pressed and the target in the payload are one answer"
            );
        }
        other => panic!("a pick asked for {other:?}"),
    }
    // The separator takes no press, and neither does anywhere else.
    let band = card.rule().expect("a rule is drawn").center();
    assert!(matches!(
        bay.chose(
            Point {
                x: band.x,
                y: band.y
            },
            &choices
        ),
        Some(karakuri_console::view::Chose::Shut)
    ));
    let away = Point { x: 1.0, y: 1.0 };
    assert!(
        matches!(
            bay.chose(away, &choices),
            Some(karakuri_console::view::Chose::Shut)
        ),
        "a press anywhere else while the card is down is the dismissal"
    );
    // A press on a cell while the card is open dismisses the chooser rather than toggling a step.
    let cell = bay.rows[0].cells[3].center();
    let on_cell = Point {
        x: cell.x,
        y: cell.y,
    };
    assert!(
        matches!(
            bay.chose(on_cell, &choices),
            Some(karakuri_console::view::Chose::Shut)
        ),
        "a press on a cell while the card is down is the dismissal and not a step"
    );
    assert!(
        bay.press(on_cell).is_some(),
        "and the cell is still a control, which is why the window has to ask in the right order"
    );
}

/// A card that is down claims every press on the console, which is
/// `input::claim`'s rule 2 with a fifth card added to it.
#[test]
fn the_card_claims_every_press_while_it_is_down() {
    let ctx = drawn_once();
    let mut panel = arranged(PLAUSIBLE, (1920, 1080));
    let mut view = choosing();
    let away = Point { x: 4.0, y: 4.0 };
    assert_eq!(
        claim(&mut panel, &ctx, &view, away),
        Claim::Egui,
        "the top-left corner is nobody's while every card is up"
    );
    assert!(view.open_lane());
    assert_eq!(
        claim(&mut panel, &ctx, &view, away),
        Claim::Panel,
        "and it is the panel's while the chooser is down"
    );
    assert!(view.shut_lane());
    assert_eq!(claim(&mut panel, &ctx, &view, away), Claim::Egui);
}

/// A chooser with nothing in it does not open, which is the deck pulldown's
/// refusal: a card with no items is a gesture with nothing to pick and nothing
/// to leave by, and rule 2 would give it every press on the console until a
/// second one shut it.
#[test]
fn a_chooser_with_nothing_to_point_at_refuses_to_open() {
    let mut view = view(StepMode::Sixteenth, Some(0));
    assert!(view.lane_choices().items.is_empty());
    assert!(
        !view.open_lane(),
        "no strips and no panes is nothing to list"
    );
    assert!(!view.lane_open());
}

/// Verifies that the sequencer bay minimum height accommodates one lane and the footer.
#[test]
fn the_reserved_minimum_holds_one_lane_and_the_foot() {
    let ctx = drawn_once();
    let panel = arranged(common::SMALLEST, (1920, 1080));
    let region = rect_of(panel.layout(), "sequencer");
    let view = view(StepMode::Sixteenth, Some(0));
    let bay = sequencer(
        &ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
    .expect("the arrangement reserves a sequencer of one lane and a foot");
    assert!(
        bay.add.max.y <= region.y + region.h,
        "the `+ lane` pill is inside the bay it is the foot of"
    );
    assert!(
        bay.rows[0].cells[0].max.y <= bay.add.min.y,
        "and the one lane clears it"
    );
    // **And the four bank pills are still in the head there**, which is the
    // other half of what this bay needs room for: `bank_capsules` hands back
    // none rather than some when the head cannot hold all four, so a narrow
    // arrangement that lost them would lose every bank press at once.
    assert_eq!(
        bay.banks.len(),
        BANKS,
        "the bay head holds its four bank pills at the smallest window"
    );
    // **The height one lane and the foot actually want**, measured off the
    // drawing rather than transcribed — the rows from the top of the region,
    // the gap over the foot, the pill and the bay's bottom padding. `lib.rs`'s
    // `min` for this region has to be at least this, and it is.
    let needed = bay.rows[0].cells[0].max.y - region.y
        + size::SEQ_STACK_GAP
        + size::PILL_H
        + size::SEQ_PAD_X;
    println!("MEASURED needed={needed} region.h={}", region.h);
    assert!(
        needed <= region.h,
        "one lane and the foot want {needed} and the region is {}",
        region.h
    );
}
