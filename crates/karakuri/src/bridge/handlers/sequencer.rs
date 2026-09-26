use super::*;

/// Returns initial sequencer banks with a muted fader lane on deck A in bank 0 (ADR-0320, ADR-0322).
pub(crate) fn demonstration_banks() -> karakuri_pattern::Banks {
    let mut banks = karakuri_pattern::Banks::default();
    let mut lane =
        karakuri_pattern::Lane::new(karakuri_operation::LaneTarget::Fader { deck: 0 }, 1.0, 0.0);
    lane.set_muted(true);
    if let Some(bank) = banks.at_mut(0) {
        bank.push(lane);
    }
    banks
}

/// Appends a pattern lane targeting a parameter or channel fader, resolving on/off values from the view (ADR-0320, ADR-0321, ADR-0327).
pub(crate) fn pointed_lane(
    banks: &mut karakuri_pattern::Banks,
    view: &View,
    pattern: u8,
    target: &karakuri_operation::LaneTarget,
) -> String {
    let levels = match target {
        karakuri_operation::LaneTarget::Fader { .. } => Some((1.0, 0.0)),
        karakuri_operation::LaneTarget::Param { deck, param } => view
            .inspector
            .iter()
            .filter(|pane| pane.deck == usize::from(*deck))
            .flat_map(|pane| pane.nodes.iter())
            .flat_map(|node| node.params.iter())
            .find(|row| &row.param == param)
            .map(|row| (row.range[1], row.range[0])),
    };
    let Some((on, off)) = levels else {
        return format!(
            "  lane: pattern {pattern} is not pointed — this console holds no published range \
             for that control, and a lane's two levels are the range published to it at the \
             press. Point the Library bay's load pulldown at the deck whose control it is and \
             ask again"
        );
    };
    let Some(bank) = banks.at_mut(usize::from(pattern)) else {
        return format!("  lane: pattern {pattern} is not a bank this session holds");
    };
    // Newly created lanes arrive muted so they do not immediately override channel values.
    let mut lane = karakuri_pattern::Lane::new(target.clone(), on, off);
    lane.set_muted(true);
    bank.push(lane);
    let lanes = bank.lanes().len();
    format!(
        "  lane: pattern {pattern} -> a {} lane, on {on} off {off} -> no record. {lanes} lane{} \
         under the rows, muted: every slot is off and an off step writes {off}, so the label is \
         the press that starts it",
        match target {
            karakuri_operation::LaneTarget::Fader { .. } => "fader",
            karakuri_operation::LaneTarget::Param { .. } => "parameter",
        },
        match lanes == 1 {
            true => "",
            false => "s",
        }
    )
}

/// Applies sequencer operations to banks and playhead, returning diagnostic text (ADR-0320, ADR-0322).
///
/// Returns `None` for operations not belonging to the Sequencer bay.
pub(crate) fn sequenced(
    banks: &mut karakuri_pattern::Banks,
    playhead: &mut karakuri_pattern::Playhead,
    view: &View,
    operation: &Operation,
) -> Option<String> {
    if let Operation::PointLane { pattern, target } = operation {
        return Some(pointed_lane(banks, view, *pattern, target));
    }
    match *operation {
        Operation::SetStep {
            pattern,
            lane,
            step,
            on,
        } => {
            let Some(lane_at) = banks
                .at_mut(usize::from(pattern))
                .and_then(|at| at.lane_mut(usize::from(lane)))
            else {
                return Some(format!(
                    "  step: pattern {pattern} lane {lane} is not a lane this session holds — a \
                     press names a bank, a lane and a slot, and this one names no row"
                ));
            };
            lane_at.set_slot(usize::from(step), on);
            Some(format!(
                "  step: pattern {pattern} lane {lane} slot {step} -> {} -> no record, and that \
                 is settled: a pattern is library data and what a lane does is its own writes. \
                 Heard the next time the playhead reaches it",
                match on {
                    true => "on",
                    false => "off",
                }
            ))
        }
        Operation::SetLaneMute {
            pattern,
            lane,
            muted,
        } => {
            let Some(lane_at) = banks
                .at_mut(usize::from(pattern))
                .and_then(|at| at.lane_mut(usize::from(lane)))
            else {
                return Some(format!(
                    "  lane: pattern {pattern} lane {lane} is not a lane this session holds"
                ));
            };
            lane_at.set_muted(muted);
            Some(format!(
                "  lane: pattern {pattern} lane {lane} -> {} -> no record. {}",
                match muted {
                    true => "muted",
                    false => "driving",
                },
                match muted {
                    true =>
                        "The pattern is kept and drives nothing, and the fader is a hand's \
                             again — which is where this lane's take-back sits",
                    false => "It writes its target at the next step boundary",
                }
            ))
        }
        // Removes the lane at the specified index without resetting the playhead.
        Operation::RemoveLane { pattern, lane } => {
            let Some(bank) = banks.at_mut(usize::from(pattern)) else {
                return Some(format!(
                    "  lane: pattern {pattern} is not a bank this session holds"
                ));
            };
            let held = bank.lanes().len();
            let Some(taken) = bank.remove(usize::from(lane)) else {
                // `karakuri_environment::no_such_slot`'s sentence, one address
                // along: the thing named, then what there was to name.
                return Some(match held {
                    0 => format!(
                        "  lane: no lane {lane} in pattern {pattern}: this pattern holds none"
                    ),
                    n => format!(
                        "  lane: no lane {lane} in pattern {pattern}: this pattern holds lanes \
                         0-{}",
                        n - 1
                    ),
                });
            };
            let left = bank.lanes().len();
            Some(format!(
                "  lane: pattern {pattern} lane {lane} taken out -> no record. Its steps went \
                 with it and the lanes after it moved up, so {left} lane{} {} under the rows. \
                 {}",
                match left == 1 {
                    true => "",
                    false => "s",
                },
                match left == 1 {
                    true => "is",
                    false => "are",
                },
                match taken.muted() {
                    true => "It was muted and drove nothing",
                    false =>
                        "It was driving, and what it drove keeps the value its last step wrote \
                         it: a lane's writes are its whole record",
                }
            ))
        }
        Operation::SetPatternGrid { pattern, grid } => {
            let Some(at) = banks.at_mut(usize::from(pattern)) else {
                return Some(format!(
                    "  grid: pattern {pattern} is not a bank this session holds"
                ));
            };
            at.set_mode(grid);
            // The same bar at another width, so the index it was remembering
            // is about a reading that has gone.
            playhead.reset();
            Some(format!(
                "  grid: pattern {pattern} -> {} -> no record. The bar is one bar, so the count \
                 follows: {} steps over the same row, and the sixteen slots underneath are \
                 untouched",
                grid.name(),
                grid.count()
            ))
        }
        Operation::SelectPattern { pattern } => {
            if !banks.select(usize::from(pattern)) {
                return Some(format!(
                    "  pattern: there is no bank {pattern} — this session holds {}",
                    karakuri_pattern::BANKS
                ));
            }
            playhead.reset();
            Some(format!(
                "  pattern: bank {pattern} armed -> no record. {} lane{} under the rows",
                banks.pattern().lanes().len(),
                match banks.pattern().lanes().len() == 1 {
                    true => "",
                    false => "s",
                }
            ))
        }
        _ => None,
    }
}
