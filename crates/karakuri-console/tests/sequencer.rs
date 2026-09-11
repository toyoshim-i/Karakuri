//! **The Sequencer bay: the console's first controls over authored state, and
//! the first bay whose body is a value no engine holds.**
//!
//! Seven things, and the first three are what the four records of 2026-09-09
//! decided:
//!
//! 1. Where the bay's parts are, measured off its own rectangle — the mock's
//!    `.seq` padding under the bay head, the head, the ruler and a row per
//!    lane.
//! 2. **That a cell press is a state and never a flip**, which is the mock's
//!    own sentence and the reason `Operation::SetStep` carries `on: bool`: a
//!    surface that could only flip has no way to arrive.
//! 3. **That a cell sends a stored slot and not a drawn step** — the identity
//!    at a sixteenth and `2k` at an eighth, so a step press and a mode press
//!    cannot race into an address that means two things (ADR-0320).
//! 4. That the label mutes the lane and the mode pill names the other mode.
//! 5. That every press names the bank it landed on rather than implying the
//!    armed one.
//! 6. **What the bay declares** (ADR-0283, ADR-0322): the step's staleness
//!    while there is a lane to move a playhead over, `moves_in` from where the
//!    beat has got to, and the invariant between them.
//! 7. That a console with no pattern behind it draws nothing and claims no
//!    press.
//!
//! None of it needs a window, a device or a disk. It does need `egui`'s fonts,
//! because the head's readout is as wide as the words in it — see
//! `common::drawn_once`.

mod common;

use std::time::Duration;

use common::{arranged, drawn_once, rect_of, solved, PLAUSIBLE};
use karakuri_console::input::{claim, Claim};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    sequencer, step_moves_in, Sequenced, Transport, View, STEP_STALENESS,
};
use karakuri_layout::Point;
use karakuri_operation::{LaneTarget, Operation, StepMode};
use karakuri_pattern::{Lane, Pattern, BANKS, SLOTS};

/// **The mock's own first lane**: deck A's channel fader, nearly full with two
/// gaps — `docs/manual/console.html` draws sixteen cells with steps 6, 7, 9
/// and 10 off, and what matters here is that some are on and some are not.
fn lane_a() -> Lane {
    let mut lane = Lane::new(LaneTarget::Fader { deck: 0 }, 1.0, 0.0);
    for slot in [0, 1, 2, 3, 4, 5, 8, 10, 11, 12, 13, 14, 15] {
        lane.set_slot(slot, true);
    }
    lane
}

/// A pattern with that lane in it, at `mode`.
fn pattern(mode: StepMode) -> Pattern {
    let mut pattern = Pattern::empty();
    pattern.set_mode(mode);
    pattern.push(lane_a());
    pattern
}

/// A view with a pattern in front of it and the playhead at `step` — what
/// `View::draw` paints from and what `claim` hit-tests, one value.
fn view(mode: StepMode, step: Option<usize>) -> View {
    let mut view = View::new(Room::Day);
    view.sequencer = Some(Sequenced {
        pattern: pattern(mode),
        bank: 0,
        step,
    });
    view
}

/// **The bay is laid out under its own head, and every row is inside it.**
#[test]
fn the_bay_is_laid_out_under_its_head_and_stays_inside_the_card() {
    let ctx = drawn_once();
    let panel = arranged(PLAUSIBLE, (1920, 1080));
    let view = view(StepMode::Sixteenth, Some(5));
    let bay = sequencer(
        &ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
    .expect("a bay with a pattern behind it draws");
    let region = rect_of(panel.layout(), "sequencer");
    assert!(
        bay.mode_pill.min.y >= region.y + size::HEAD_H,
        "the head's pill sits under the bay head, not in it"
    );
    assert!(
        bay.mode_pill.min.x >= region.x + size::SEQ_PAD_X,
        "and inside `.seq`'s padding"
    );
    let row = &bay.rows[0];
    assert_eq!(
        row.cells.len(),
        SLOTS,
        "a sixteenth draws sixteen cells, which is the bar"
    );
    assert!(
        row.label.max.x <= row.cells[0].min.x,
        "the label column is before the cells and does not overlap the first one"
    );
    assert!(
        row.cells.last().unwrap().max.x <= region.x + region.w - size::SEQ_PAD_X,
        "and the last cell stops at the bay's own padding"
    );
    assert!(
        bay.ruler.max.y <= row.cells[0].min.y,
        "the ruler is over the rows it counts"
    );
    // **What this bay claims**, which is what `input::PROBES` registers a
    // bound for: a cell per drawn step, a label per lane, the mode pill, the
    // four bank pills in the bay head and the foot's `+ lane`.
    assert_eq!(
        bay.controls(),
        SLOTS + 2 + BANKS + 1,
        "one lane of sixteen cells, its label, the pill, four banks and + lane"
    );
}

/// **The count follows the mode**: eight cells at an eighth, over the same
/// width — *"the row keeps its width, so the cells halve in the finer one"*
/// read the other way round (ADR-0306).
#[test]
fn an_eighth_draws_eight_cells_over_the_same_row() {
    let ctx = drawn_once();
    let panel = arranged(PLAUSIBLE, (1920, 1080));
    let fine = view(StepMode::Sixteenth, None);
    let coarse = view(StepMode::Eighth, None);
    let fine = sequencer(
        &ctx,
        panel.layout(),
        fine.sequencer.as_ref(),
        &fine.lane_choices(),
    )
    .unwrap();
    let coarse = sequencer(
        &ctx,
        panel.layout(),
        coarse.sequencer.as_ref(),
        &coarse.lane_choices(),
    )
    .unwrap();
    assert_eq!(fine.rows[0].cells.len(), 16);
    assert_eq!(coarse.rows[0].cells.len(), 8);
    let fine_row = &fine.rows[0];
    let coarse_row = &coarse.rows[0];
    assert_eq!(
        fine_row.cells[0].min.x, coarse_row.cells[0].min.x,
        "both rows start at the same place: the row is one bar either way"
    );
    assert!(
        coarse_row.cells[0].width() > fine_row.cells[0].width(),
        "and an eighth's cells are the wider ones"
    );
}

/// **An eighth reads slot `2k`, and a press says so.** The console sends the
/// stored slot, so the payload means the same thing whichever mode the head is
/// in.
#[test]
fn a_cell_press_sends_the_stored_slot_and_not_the_drawn_step() {
    let ctx = drawn_once();
    let panel = arranged(PLAUSIBLE, (1920, 1080));
    let view = view(StepMode::Eighth, None);
    let bay = sequencer(
        &ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
    .unwrap();
    let row = &bay.rows[0];
    assert_eq!(
        row.slots,
        vec![0, 2, 4, 6, 8, 10, 12, 14],
        "an eighth's eight cells are the even slots of the sixteen"
    );
    let third = row.cells[3].center();
    let asked = bay
        .press(Point {
            x: third.x,
            y: third.y,
        })
        .expect("a press in a cell asks for a step");
    match asked {
        Operation::SetStep { step, .. } => assert_eq!(
            step, 6,
            "the fourth cell of an eighth is stored slot 6, and sending 3 would be an address \
             that means one thing at one mode and another at the other"
        ),
        other => panic!("a press in a cell asked for {other:?}"),
    }
}

/// **A press asks for the state the cell is not in**, on a lit cell and on an
/// unlit one — a state and never a flip, and it names the bank and the lane.
#[test]
fn a_cell_press_is_a_state_and_names_the_bank_and_the_lane() {
    let ctx = drawn_once();
    let panel = arranged(PLAUSIBLE, (1920, 1080));
    let view = view(StepMode::Sixteenth, None);
    let bay = sequencer(
        &ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
    .unwrap();
    let row = &bay.rows[0];
    // Slot 0 is on in the mock's lane and slot 6 is one of its two gaps.
    for (cell, on) in [(0usize, false), (6usize, true)] {
        assert_eq!(row.on[cell], !on, "the fixture is what this test is about");
        let at = row.cells[cell].center();
        assert_eq!(
            bay.press(Point { x: at.x, y: at.y }),
            Some(Operation::SetStep {
                pattern: 0,
                lane: 0,
                step: cell as u8,
                on,
            }),
            "a press on a {} cell asks for it to be {}",
            match on {
                true => "dark",
                false => "lit",
            },
            match on {
                true => "on",
                false => "off",
            }
        );
    }
}

/// **The label mutes the lane and the mode pill names the other mode**, and
/// both name the bank.
#[test]
fn the_label_mutes_and_the_pill_names_the_other_mode() {
    let ctx = drawn_once();
    let panel = arranged(PLAUSIBLE, (1920, 1080));
    let view = view(StepMode::Sixteenth, None);
    let bay = sequencer(
        &ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
    .unwrap();
    let label = bay.rows[0].label.center();
    assert_eq!(
        bay.press(Point {
            x: label.x,
            y: label.y
        }),
        Some(Operation::SetLaneMute {
            pattern: 0,
            lane: 0,
            muted: true,
        }),
        "a press on a driving lane's label asks for it to be muted, which is where this lane's \
         take-back sits"
    );
    let pill = bay.mode_pill.center();
    assert_eq!(
        bay.press(Point {
            x: pill.x,
            y: pill.y
        }),
        Some(Operation::SetPatternGrid {
            pattern: 0,
            grid: StepMode::Eighth,
        }),
        "the pill reads 1/16 and a press asks for the other of the two by naming it"
    );
}

/// **A muted lane's label asks for it back**, which is the other half of the
/// same state.
#[test]
fn a_muted_lanes_label_asks_for_it_to_drive() {
    let ctx = drawn_once();
    let panel = arranged(PLAUSIBLE, (1920, 1080));
    let mut view = View::new(Room::Day);
    let mut pattern = Pattern::empty();
    let mut lane = lane_a();
    lane.set_muted(true);
    pattern.push(lane);
    view.sequencer = Some(Sequenced {
        pattern,
        bank: 2,
        step: None,
    });
    let bay = sequencer(
        &ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
    .unwrap();
    let label = bay.rows[0].label.center();
    assert_eq!(
        bay.press(Point {
            x: label.x,
            y: label.y
        }),
        Some(Operation::SetLaneMute {
            pattern: 2,
            lane: 0,
            muted: false,
        }),
        "the bank is the one these rows are, and not whichever one is armed by the time the \
         press is performed"
    );
}

/// **The playhead stands over the cell the poll answered**, and over nothing
/// before the first poll.
#[test]
fn the_playhead_stands_over_the_step_the_poll_answered() {
    let ctx = drawn_once();
    let panel = arranged(PLAUSIBLE, (1920, 1080));
    let polled = view(StepMode::Sixteenth, Some(5));
    let bay = sequencer(
        &ctx,
        panel.layout(),
        polled.sequencer.as_ref(),
        &polled.lane_choices(),
    )
    .unwrap();
    let column = bay.playhead.expect("a polled bay draws its playhead");
    let cell = bay.rows[0].cells[5];
    assert_eq!(
        (column.min.x, column.max.x),
        (cell.min.x, cell.max.x),
        "the column is the cell's width, over the step the producer last emitted"
    );
    let unpolled = view(StepMode::Sixteenth, None);
    let unpolled = sequencer(
        &ctx,
        panel.layout(),
        unpolled.sequencer.as_ref(),
        &unpolled.lane_choices(),
    )
    .unwrap();
    assert!(
        unpolled.playhead.is_none(),
        "a bay nobody has polled is not a bay at step zero"
    );
}

/// **The bay declares while it has a lane, and the two numbers obey the
/// invariant** — ADR-0283's `moves_in >= staleness`, which this is the first
/// region to be tight against.
#[test]
fn the_bay_declares_a_step_and_the_deadline_never_beats_the_rate() {
    let layout = solved(PLAUSIBLE);
    let mut declaring = view(StepMode::Sixteenth, Some(0));
    declaring.transport = Some(Transport {
        bpm: 128.0,
        beats: 0.0,
        beats_per_bar: 4,
        fps: None,
        frame_ms: 12.4,
        budget_ms: None,
        health: None,
        rec: None,
    });
    let declared: Vec<_> = declaring.declares(&layout).collect();
    let seq = declared
        .iter()
        .find(|d| d.region == "sequencer")
        .expect("a bay with a lane in it declares");
    assert_eq!(seq.staleness, STEP_STALENESS);
    assert!(
        seq.moves_in >= seq.staleness,
        "change detection may take a frame away and may never bring one forward"
    );
    // A sixteenth at 128 BPM is 117.19 ms, and a beat exactly on a boundary
    // has a whole step to the next one.
    assert!(
        (seq.moves_in.as_secs_f64() - 0.1171875).abs() < 1e-6,
        "a step at the mock's tempo is 117.19 ms and this deadline is {:?}",
        seq.moves_in
    );
    // Anywhere inside a step the deadline is shorter, and never shorter than
    // the rate the bay declared.
    for beats in [0.01, 0.2, 0.24999, 3.9999] {
        let left = step_moves_in(StepMode::Sixteenth, beats, 128.0);
        assert!(
            left >= STEP_STALENESS && left <= Duration::from_millis(118),
            "{beats} beats is {left:?}, which is not inside one step of the rate"
        );
    }
}

/// **A pattern with no lanes declares nothing**, because the playhead has
/// nothing to stand over: a bay whose picture does not change is a bay a
/// declaration would buy frames of nothing for (ADR-0283).
#[test]
fn a_pattern_with_no_lanes_declares_nothing() {
    let layout = solved(PLAUSIBLE);
    let mut view = View::new(Room::Day);
    view.transport = Some(Transport {
        bpm: 128.0,
        beats: 0.0,
        beats_per_bar: 4,
        fps: None,
        frame_ms: 12.4,
        budget_ms: None,
        health: None,
        rec: None,
    });
    view.sequencer = Some(Sequenced {
        pattern: Pattern::empty(),
        bank: 0,
        step: Some(0),
    });
    assert!(
        !view.declares(&layout).any(|d| d.region == "sequencer"),
        "an empty bank draws a head and a ruler, and neither of them moves"
    );
}

/// **A console with no pattern behind it draws nothing and claims no press**,
/// which is every test in this crate that does not hand one in.
#[test]
fn no_pattern_is_no_bay_and_no_press() {
    let ctx = drawn_once();
    let mut panel = arranged(PLAUSIBLE, (1920, 1080));
    let view = View::new(Room::Day);
    assert!(
        sequencer(
            &ctx,
            panel.layout(),
            view.sequencer.as_ref(),
            &view.lane_choices()
        )
        .is_none(),
        "no pattern is no bay at all, and not an empty grid"
    );
    let region = rect_of(panel.layout(), "sequencer");
    let at = Point {
        x: region.x + region.w * 0.5,
        y: region.y + region.h * 0.5,
    };
    assert_eq!(
        claim(&mut panel, &ctx, &view, at),
        Claim::Egui,
        "and nothing in an empty bay takes a press off `egui`"
    );
}

/// **The route a window loop actually takes**: `claim` says the press is the
/// panel's, and the derivation that drew the cell is the one that answers it.
#[test]
fn a_press_on_a_cell_is_claimed_and_then_answered() {
    let ctx = drawn_once();
    let mut panel = arranged(PLAUSIBLE, (1920, 1080));
    let view = view(StepMode::Sixteenth, Some(2));
    let bay = sequencer(
        &ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
    .unwrap();
    let cell = bay.rows[0].cells[6].center();
    let at = Point {
        x: cell.x,
        y: cell.y,
    };
    assert_eq!(
        claim(&mut panel, &ctx, &view, at),
        Claim::Panel,
        "a cell is a control, so the press is the panel's"
    );
    assert!(
        sequencer(
            &ctx,
            panel.layout(),
            view.sequencer.as_ref(),
            &view.lane_choices()
        )
        .expect("the same derivation the frame drew")
        .press(at)
        .is_some(),
        "and the derivation that drew it is what answers the press"
    );
}

/// **The bay draws at the smallest window this arrangement is claimed to work
/// at**, which is what `lib.rs`'s minimum for it reserves — *"a sequencer of
/// one lane"*. A bay that answered `None` there would be a lane an operator
/// could reach on one window and not on another, with nothing saying so.
#[test]
fn one_lane_draws_at_the_smallest_window() {
    let ctx = drawn_once();
    let panel = arranged(common::SMALLEST, (1920, 1080));
    let view = view(StepMode::Sixteenth, Some(0));
    let bay = sequencer(
        &ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
    .expect("the arrangement reserves a sequencer of one lane, so one lane draws");
    assert_eq!(bay.rows.len(), 1);
    assert_eq!(bay.rows[0].cells.len(), SLOTS);
    assert!(
        bay.rows[0].cells[0].width() > 0.0,
        "and every cell has a width, which is what makes it a control"
    );
}

// ---------------------------------------------------------------------------
// The bay head's bank pills
// ---------------------------------------------------------------------------

/// **Four pills in the bay head, and the armed one is marked.**
///
/// Four rather than the mock's `seq 1 · seq 2 · +`: with four fixed banks the
/// `+` is `SelectPattern` landing on an empty bank, which is what a press on
/// `seq 3` already is (ADR-0320, ADR-0327). The marking is `HeadWords::armed`,
/// which is the head machinery's half of this and is `tests/head_words.rs`'s.
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

/// **A press on a bank pill asks for that bank, and it is a state.**
///
/// Never *the next one* and never a cycle: a map with a button per bank has to
/// be able to say *this bank* and mean it, which is the cell's own rule one
/// control down.
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

/// **The chooser lists every drawn deck's fader and one deck's published
/// controls**, and the deck is the Library bay's load pulldown's (ADR-0305,
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

/// **A press on `+ lane` puts the card down; a pick points the lane and names
/// the bank; every other press dismisses it.**
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
    // **Including a press on a cell**, which is the one that matters: the card
    // hangs over this bay's own rows, so a hand mid-choice reaching a cell is
    // dismissing the card rather than setting a step — and this answering
    // `Some` is what lets the window ask this control *before* `press` while
    // the card is down.
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

/// **A card that is down claims every press on the console**, which is
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

/// **A chooser with nothing in it does not open**, which is the deck
/// pulldown's refusal: a card with no items is a gesture with nothing to pick
/// and nothing to leave by, and rule 2 would give it every press on the
/// console until a second one shut it.
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

/// **The bay's own minimum is what the arrangement reserves for it**, and the
/// foot is part of it.
///
/// `lib.rs` reserves `min(100.0)` for this region with the arithmetic written
/// out beside it — *"the same with one lane and no foot"* — and the panel draws
/// a foot now, so this is the assertion that says which of the two moved. **The
/// bay is clipped rather than half drawn**, so a region at a height that cannot
/// hold the rows *and* the `+ lane` pill draws nothing at all, and a minimum
/// that had not moved would be a Sequencer bay that went blank on a short
/// window.
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
