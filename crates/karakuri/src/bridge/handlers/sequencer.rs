use super::*;

/// The four banks a run starts with: bank 0 holding one lane over deck A's
/// channel fader, muted, and three empty banks beside it.
///
/// # Why there is a lane at all before anything has added one
///
/// `Operation::PointLane` is the control that makes a lane and the console
/// draws none — the mock's `+ lane` needs a target chooser nobody has drawn,
/// and the bay cannot grow a body while it is running anyway. So a first slice
/// with no lane would draw a ruler over nothing and there would be no way to
/// reach a cell (ADR-0320's consequences, and `view::sequencer`'s own list of
/// what is not drawn). This is the demonstration, written here rather than in
/// `karakuri-pattern` because it is *this program's opening state* and not what
/// a pattern is.
///
/// # Why it is muted
///
/// An unmuted lane writes its target on every step boundary, on-steps and
/// off-steps alike — that is what makes a lane a gate rather than a set of
/// impulses (ADR-0320). A lane over deck A's fader with every step off would
/// therefore hold deck A at `off` from the moment the window opened: the deck
/// on air would go dark, and nothing on screen would say a sequencer had done
/// it.
///
/// So the lane arrives the way the mock's third row is drawn — *"the pattern is
/// kept and drives nothing"* — and the first press is the one that starts it.
/// That is also the shape of the demonstration: mute the lane and the fader is
/// the hand's again, which is rule 02's take-back for a lane and what ADR-0322
/// says the mute is *for*.
///
/// `on` is 1.0 and `off` is 0.0, which is a fader's pair: a gate.
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
    // **It arrives muted, and that is not caution.** A lane arrives with every
    // slot off, and an off step writes `off` rather than writing nothing — so
    // an unmuted fader lane would hold its deck at zero from the press, which
    // is `demonstration_banks`' argument reached from the other end: the deck
    // would go dark and nothing on screen would say a sequencer had done it.
    // The label is the unmute and it is one press, which is rule 02's take-back
    // drawn where the lane is.
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

/// A press in the Sequencer bay, applied to the pattern it names, and what to
/// say about it. `None` for every operation that is not one of the six.
///
/// [`scheduled`]'s shape one bay along, and for the same reason: `written`
/// answers `Silent(Surface)` for all six of the sequencer's operations — a
/// pattern is library data under the store on the arrangement's terms, and what
/// a *lane* does reaches the stream as its own writes (ADR-0320, ADR-0322) — so
/// there is nothing on the deck for [`apply`] to move and the surface that
/// emits it is what performs it.
///
/// Every arm names its bank, and a press on a bank this session does not have
/// is refused and said rather than swallowed, which is [`pointed`]'s rule: a
/// press that does nothing and a press that is not bound are the same
/// experience.
///
/// A mode press and a bank press reset the playhead, and a step press does not.
/// The first two change what a step *index* means — the same bar read at
/// another width, or another pattern's lanes under it — so a remembered index
/// would hold the new reading silent until the bar came round. Turning a cell
/// on changes what the *current* step is worth and not which step it is, and
/// the mock already says when that is heard: *"the next time the playhead
/// reaches the cell rather than when you asked for it"*.
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
        // **A lane taken out, by the position it was drawn at.** The lanes
        // after it move up, which is what a lane index means
        // (`karakuri_pattern::Pattern::remove`), and the steps on it go with
        // it — there is nothing left to unmute onto, which is what separates
        // this from the mute above.
        //
        // The playhead is not reset, for the step press's reason: a removal
        // changes what is under the current step and not which step it is.
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
