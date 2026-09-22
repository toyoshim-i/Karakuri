use super::sequencer_common::*;

/// The bay is laid out under its own head, and every row is inside it.
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
        row.cells.last().unwrap().max.x <= row.remove.min.x,
        "the cells stop short of the minus at the end of the row"
    );
    assert!(
        row.remove.max.x <= region.x + region.w - size::SEQ_PAD_X,
        "and the minus stops at the bay's own padding"
    );
    assert!(
        bay.ruler.max.y <= row.cells[0].min.y,
        "the ruler is over the rows it counts"
    );
    // **What this bay claims**, which is what `input::PROBES` registers a
    // bound for: a cell per drawn step, a label and a minus per lane, the mode
    // pill, the four bank pills in the bay head and the foot's `+ lane`.
    assert_eq!(
        bay.controls(),
        SLOTS + 3 + BANKS + 1,
        "one lane of sixteen cells, its label, its minus, the pill, four banks and + lane"
    );
}

/// The count follows the mode: eight cells at an eighth, over the same width —
/// *"the row keeps its width, so the cells halve in the finer one"* read the
/// other way round (ADR-0306).
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

/// An eighth reads slot `2k`, and a press says so. The console sends the stored
/// slot, so the payload means the same thing whichever mode the head is in.
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

/// A press asks for the state the cell is not in, on a lit cell and on an unlit
/// one — a state and never a flip, and it names the bank and the lane.
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

/// The label mutes the lane and the mode pill names the other mode, and both
/// name the bank.
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

/// A muted lane's label asks for it back, which is the other half of the same
/// state.
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

/// The minus at the end of a lane's row takes that lane out, naming the bank
/// these rows are and the position the lane is drawn at — the Master bay's chain
/// glyph on a lane (ADR-0352).
#[test]
fn the_minus_takes_the_lane_out_by_the_position_it_is_drawn_at() {
    let ctx = drawn_once();
    let panel = arranged(PLAUSIBLE, (1920, 1080));
    let mut view = View::new(Room::Day);
    let mut pattern = Pattern::empty();
    pattern.push(lane_a());
    pattern.push(Lane::new(LaneTarget::Fader { deck: 1 }, 1.0, 0.0));
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
    for lane in 0..2 {
        let at = bay.rows[lane].remove.center();
        assert_eq!(
            bay.press(Point { x: at.x, y: at.y }),
            Some(Operation::RemoveLane {
                pattern: 2,
                lane: lane as u8,
            }),
            "the glyph on row {lane} is addressed to the lane it is drawn on, and names the \
             bank these rows are"
        );
    }
    assert!(
        bay.owns(Point {
            x: bay.rows[0].remove.center().x,
            y: bay.rows[0].remove.center().y
        }),
        "and the bay owns the press, so `input::claim` hands it to the panel"
    );
}

/// The glyph is the panel's press and not `egui`'s, which is rule 4 asked of the
/// one control this bay gained.
#[test]
fn a_press_on_a_lanes_minus_is_the_panels() {
    let ctx = drawn_once();
    let mut panel = arranged(PLAUSIBLE, (1920, 1080));
    let view = view(StepMode::Sixteenth, None);
    let bay = sequencer(
        &ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
    .unwrap();
    let at = bay.rows[0].remove.center();
    assert_eq!(
        claim(&mut panel, &ctx, &view, Point { x: at.x, y: at.y }),
        Claim::Panel,
        "a control the bay draws is hit-tested by this crate, and a press on it never reaches \
         `egui`"
    );
}

/// The minus explains itself on hover, in the page's own words — the third
/// `.minus` of `docs/manual/console.html`, the two before it being chain slots.
#[test]
fn a_lanes_minus_resolves_to_its_own_tip() {
    use karakuri_console::hover::{flat, resolve};

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
    let at = bay.rows[0].remove.center();
    let index = resolve(&panel, &ctx, &view, Point { x: at.x, y: at.y })
        .expect("the pointer is on a control the page tips");
    assert_eq!(
        flat().nth(index).expect("a tip at that index").control,
        "a lane's minus",
        "the pointer on the glyph resolves to the glyph's tip and not to the row it is in"
    );
}

/// The playhead stands over the cell the poll answered, and over nothing before
/// the first poll.
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

/// The bay declares while it has a lane, and the two numbers obey the invariant
/// — ADR-0283's `moves_in >= staleness`, which this is the first region to be
/// tight against.
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
        chain_ms: None,
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

/// A pattern with no lanes declares nothing, because the playhead has nothing
/// to stand over: a bay whose picture does not change is a bay a declaration
/// would buy frames of nothing for (ADR-0283).
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
        chain_ms: None,
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

/// A console with no pattern behind it draws nothing and claims no press, which
/// is every test in this crate that does not hand one in.
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

/// The route a window loop actually takes: `claim` says the press is the
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

/// The bay draws at the smallest window this arrangement is claimed to work at,
/// which is what `lib.rs`'s minimum for it reserves — *"a sequencer of one
/// lane"*. A bay that answered `None` there would be a lane an operator could
/// reach on one window and not on another, with nothing saying so.
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
