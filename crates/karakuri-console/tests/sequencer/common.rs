pub(crate) use super::common::{self as common, arranged, drawn_once, rect_of, solved, PLAUSIBLE};
pub(crate) use karakuri_console::input::{claim, Claim};
pub(crate) use karakuri_console::room::{size, Room};
pub(crate) use karakuri_console::view::{
    sequencer, step_moves_in, Sequenced, Transport, View, STEP_STALENESS,
};
pub(crate) use karakuri_layout::Point;
pub(crate) use karakuri_operation::{LaneTarget, Operation, StepMode};
pub(crate) use karakuri_pattern::{Lane, Pattern, BANKS, SLOTS};
pub(crate) use std::time::Duration;

/// The mock's own first lane: deck A's channel fader, nearly full with two gaps
/// — `docs/manual/console.html` draws sixteen cells with steps 6, 7, 9 and 10
/// off, and what matters here is that some are on and some are not.
pub(crate) fn lane_a() -> Lane {
    let mut lane = Lane::new(LaneTarget::Fader { deck: 0 }, 1.0, 0.0);
    for slot in [0, 1, 2, 3, 4, 5, 8, 10, 11, 12, 13, 14, 15] {
        lane.set_slot(slot, true);
    }
    lane
}

/// A pattern with that lane in it, at `mode`.
pub(crate) fn pattern(mode: StepMode) -> Pattern {
    let mut pattern = Pattern::empty();
    pattern.set_mode(mode);
    pattern.push(lane_a());
    pattern
}

/// A view with a pattern in front of it and the playhead at `step` — what
/// `View::draw` paints from and what `claim` hit-tests, one value.
pub(crate) fn view(mode: StepMode, step: Option<usize>) -> View {
    let mut view = View::new(Room::Day);
    view.sequencer = Some(Sequenced {
        pattern: pattern(mode),
        bank: 0,
        step,
    });
    view
}
