//! Sequencer pattern representation, step lanes, and bank management.
//!
//! Evaluates musical step sequences against beat counts to emit typed [`Operation`]s.
//!
//! - [`Pattern`]: Represents one bar of 16 step slots in either eighth or sixteenth note subdivisions.
//! - [`Lane`]: Associates a [`LaneTarget`] with step bitmasks and discrete on/off levels.
//! - [`Banks`]: Session container managing four pattern banks and tracking the armed bank.
//! - [`Playhead`]: Tracks step progression and emits operations only upon step boundary crossings.

use karakuri_operation::{LaneTarget, Operation, StepMode};

/// Number of step slots stored per pattern across both eighth and sixteenth modes.
pub const SLOTS: usize = 16;

/// Maximum number of pattern banks maintained per session.
pub const BANKS: usize = 4;

/// A single sequence lane driving an operation target with 16 slots and distinct on/off levels.
#[derive(Debug, Clone, PartialEq)]
pub struct Lane {
    target: LaneTarget,
    /// Step bitmask where bit `k` corresponds to slot `k`.
    steps: u16,
    on: f32,
    off: f32,
    muted: bool,
}

impl Lane {
    /// Creates a new lane targeting `target` with specified `on` and `off` levels.
    pub fn new(target: LaneTarget, on: f32, off: f32) -> Lane {
        Lane {
            target,
            steps: 0,
            on,
            off,
            muted: false,
        }
    }

    /// Returns the target driven by this lane.
    pub fn target(&self) -> &LaneTarget {
        &self.target
    }

    /// Returns the value written on an active step.
    pub fn on(&self) -> f32 {
        self.on
    }

    /// Returns the value written on an inactive step.
    pub fn off(&self) -> f32 {
        self.off
    }

    /// Returns whether this lane is muted.
    pub fn muted(&self) -> bool {
        self.muted
    }

    /// Sets the muted state of this lane.
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    /// Returns whether the stored slot at `slot` is active.
    pub fn slot_on(&self, slot: usize) -> bool {
        slot < SLOTS && self.steps & (1 << slot) != 0
    }

    /// Sets the active state for the specified stored slot.
    pub fn set_slot(&mut self, slot: usize, on: bool) {
        if slot >= SLOTS {
            return;
        }
        match on {
            true => self.steps |= 1 << slot,
            false => self.steps &= !(1 << slot),
        }
    }

    /// Returns whether the step in the given `mode` is active.
    pub fn step_on(&self, step: usize, mode: StepMode) -> bool {
        self.slot_on(mode.slot_of(step))
    }

    /// Returns the level written at the specified step and mode.
    pub fn value_at(&self, step: usize, mode: StepMode) -> f32 {
        match self.step_on(step, mode) {
            true => self.on,
            false => self.off,
        }
    }

    /// Emits the operation produced by this lane at the specified step.
    pub fn operation_at(&self, step: usize, mode: StepMode) -> Operation {
        self.target.operation(self.value_at(step, mode))
    }
}

/// A single musical bar composed of a subdivision mode and a list of sequence lanes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pattern {
    mode: StepMode,
    lanes: Vec<Lane>,
}

impl Pattern {
    /// Creates an empty pattern in sixteenths mode with no lanes.
    pub fn empty() -> Pattern {
        Pattern::default()
    }

    /// Returns the active step subdivision mode.
    pub fn mode(&self) -> StepMode {
        self.mode
    }

    /// Sets the step subdivision mode without mutating underlying slot states.
    pub fn set_mode(&mut self, mode: StepMode) {
        self.mode = mode;
    }

    /// Returns the sequence lanes.
    pub fn lanes(&self) -> &[Lane] {
        &self.lanes
    }

    /// Returns a mutable reference to the lane at `at`, if present.
    pub fn lane_mut(&mut self, at: usize) -> Option<&mut Lane> {
        self.lanes.get_mut(at)
    }

    /// Appends a new lane to the pattern.
    pub fn push(&mut self, lane: Lane) {
        self.lanes.push(lane);
    }

    /// Returns whether this pattern contains any lanes.
    pub fn is_empty(&self) -> bool {
        self.lanes.is_empty()
    }

    /// Computes the step index corresponding to the given musical beat position.
    pub fn step_at(&self, beats: f64) -> usize {
        let count = self.mode.count() as f64;
        let step = (beats * self.mode.steps_per_beat())
            .floor()
            .rem_euclid(count);
        step as usize
    }

    /// Returns an iterator of operations emitted by unmuted lanes at `step`.
    pub fn due(&self, step: usize) -> impl Iterator<Item = Operation> + '_ {
        let mode = self.mode;
        self.lanes
            .iter()
            .filter(|lane| !lane.muted())
            .map(move |lane| lane.operation_at(step, mode))
    }

    /// Returns the index of the lane driving `deck`'s channel fader, if any.
    pub fn holder_of_fader(&self, deck: u8) -> Option<usize> {
        self.lanes.iter().position(|lane| {
            !lane.muted() && matches!(lane.target(), LaneTarget::Fader { deck: at } if *at == deck)
        })
    }
}

/// Four pattern banks with an active armed index.
#[derive(Debug, Clone, PartialEq)]
pub struct Banks {
    banks: [Pattern; BANKS],
    armed: usize,
}

impl Default for Banks {
    /// Creates four empty banks with bank 0 armed.
    fn default() -> Banks {
        Banks {
            banks: Default::default(),
            armed: 0,
        }
    }
}

impl Banks {
    /// Returns the index of the currently armed bank.
    pub fn armed(&self) -> usize {
        self.armed
    }

    /// Selects the active pattern bank, returning `false` if `bank >= BANKS`.
    pub fn select(&mut self, bank: usize) -> bool {
        if bank >= BANKS {
            return false;
        }
        self.armed = bank;
        true
    }

    /// Returns a reference to the armed pattern.
    pub fn pattern(&self) -> &Pattern {
        &self.banks[self.armed]
    }

    /// Returns a reference to the bank at index `bank`.
    pub fn at(&self, bank: usize) -> Option<&Pattern> {
        self.banks.get(bank)
    }

    /// Returns a mutable reference to the bank at index `bank`.
    pub fn at_mut(&mut self, bank: usize) -> Option<&mut Pattern> {
        self.banks.get_mut(bank)
    }

    /// Returns the index of the first empty bank, or `None` if all banks contain lanes.
    pub fn first_empty(&self) -> Option<usize> {
        self.banks.iter().position(Pattern::is_empty)
    }
}

/// Tracks playhead step advancement and detects step boundary transitions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Playhead {
    last: Option<usize>,
}

impl Playhead {
    /// Advances the playhead given the pattern and beat count, returning `Some(step)` on boundary transitions.
    pub fn advance(&mut self, pattern: &Pattern, beats: f64) -> Option<usize> {
        let step = pattern.step_at(beats);
        match self.last {
            Some(last) if last == step => None,
            _ => {
                self.last = Some(step);
                Some(step)
            }
        }
    }

    /// Returns the current step index, or `None` if the playhead has not yet been polled.
    pub fn at(&self) -> Option<usize> {
        self.last
    }

    /// Resets playhead history, forcing the next poll to register as a boundary transition.
    pub fn reset(&mut self) {
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use karakuri_operation::{ParamAt, ParamValue};

    fn fader(deck: u8) -> Lane {
        Lane::new(LaneTarget::Fader { deck }, 1.0, 0.0)
    }

    /// Verifies step subdivision mapping and wrapping at the bar boundary.
    #[test]
    fn a_step_is_the_beat_count_subdivided_and_wraps_at_the_bar() {
        let pattern = Pattern::empty();
        for (beats, step) in [
            (0.0, 0),
            (0.24, 0),
            (0.25, 1),
            (0.5, 2),
            (1.0, 4),
            (3.75, 15),
            (4.0, 0),
            (4.25, 1),
        ] {
            assert_eq!(
                pattern.step_at(beats),
                step,
                "{beats} beats is step {step} of a bar of sixteenths"
            );
        }
    }

    /// Verifies that eighth-note evaluation reads every second slot (2k) of the 16 stored slots.
    #[test]
    fn an_eighth_reads_every_second_slot() {
        let mut lane = fader(0);
        // Slot 4 on and slot 5 off: at an eighth, step 2 is slot 4.
        lane.set_slot(4, true);
        assert!(
            lane.step_on(4, StepMode::Sixteenth),
            "slot 4 is step 4 of 16"
        );
        assert!(lane.step_on(2, StepMode::Eighth), "slot 4 is step 2 of 8");
        assert!(
            !lane.step_on(4, StepMode::Eighth),
            "step 4 of 8 is slot 8, which nothing set"
        );
        for step in 0..StepMode::Eighth.count() {
            assert_eq!(
                StepMode::Eighth.slot_of(step),
                step * 2,
                "an eighth's step {step} is slot {}",
                step * 2
            );
        }
    }

    /// Verifies that toggling subdivision modes preserves all stored slot states.
    #[test]
    fn a_mode_press_keeps_every_slot() {
        let mut pattern = Pattern::empty();
        let mut lane = fader(0);
        for slot in [1, 3, 4, 9, 15] {
            lane.set_slot(slot, true);
        }
        pattern.push(lane);
        let before = pattern.clone();
        pattern.set_mode(StepMode::Eighth);
        pattern.set_mode(StepMode::Sixteenth);
        assert_eq!(
            pattern, before,
            "a mode press must not move a step: the eighth reads slot 2k and the sixteenth reads \
             all sixteen, so going and coming back is two readings of one pattern"
        );
    }

    /// Verifies that inactive steps emit the explicit `off` level rather than being skipped.
    #[test]
    fn an_off_step_writes_the_off_level() {
        let mut lane = Lane::new(LaneTarget::Fader { deck: 0 }, 0.8, 0.5);
        lane.set_slot(0, true);
        assert_eq!(lane.value_at(0, StepMode::Sixteenth), 0.8);
        assert_eq!(
            lane.value_at(1, StepMode::Sixteenth),
            0.5,
            "an off step writes `off`; a lane that wrote only its on-steps would leave the fader \
             wherever the last on-step put it"
        );
        assert_eq!(
            lane.operation_at(1, StepMode::Sixteenth),
            Operation::SetOpacity {
                deck: 0,
                opacity: 0.5
            }
        );
    }

    /// Verifies that muted lanes emit no operations while preserving their step configurations.
    #[test]
    fn a_muted_lane_emits_nothing_and_keeps_its_steps() {
        let mut pattern = Pattern::empty();
        let mut lane = fader(0);
        lane.set_slot(0, true);
        lane.set_muted(true);
        pattern.push(lane);
        assert_eq!(
            pattern.due(0).count(),
            0,
            "a muted lane drives nothing, which is the whole of what the mute is"
        );
        assert!(
            pattern.lanes()[0].slot_on(0),
            "and the pattern is kept: the steps are still there to unmute onto"
        );
        pattern.lane_mut(0).unwrap().set_muted(false);
        assert_eq!(
            pattern.due(0).collect::<Vec<_>>(),
            vec![Operation::SetOpacity {
                deck: 0,
                opacity: 1.0
            }]
        );
    }

    /// Verifies that the playhead triggers boundary events exactly once per step.
    #[test]
    fn the_playhead_answers_once_per_step() {
        let pattern = Pattern::empty();
        let mut playhead = Playhead::default();
        assert_eq!(
            playhead.advance(&pattern, 0.0),
            Some(0),
            "the first poll is a boundary: nothing has been emitted for any step yet"
        );
        assert_eq!(
            playhead.advance(&pattern, 0.1),
            None,
            "a frame inside a step is not a boundary"
        );
        assert_eq!(playhead.advance(&pattern, 0.24), None);
        assert_eq!(playhead.advance(&pattern, 0.25), Some(1));
        assert_eq!(playhead.advance(&pattern, 0.26), None);
    }

    /// Verifies that a frame stall skips intermediate steps without emitting catch-up events.
    #[test]
    fn a_stall_drops_the_step_it_missed() {
        let pattern = Pattern::empty();
        let mut playhead = Playhead::default();
        assert_eq!(playhead.advance(&pattern, 0.0), Some(0));
        assert_eq!(
            playhead.advance(&pattern, 0.75),
            Some(3),
            "steps 1 and 2 went by inside one frame and this is step 3, once"
        );
        assert_eq!(playhead.advance(&pattern, 0.76), None);
    }

    /// Verifies that resetting the playhead treats the subsequent evaluation as a step boundary.
    #[test]
    fn a_reset_makes_the_next_poll_a_boundary() {
        let pattern = Pattern::empty();
        let mut playhead = Playhead::default();
        assert_eq!(playhead.advance(&pattern, 0.0), Some(0));
        assert_eq!(playhead.advance(&pattern, 0.0), None);
        playhead.reset();
        assert_eq!(
            playhead.advance(&pattern, 0.0),
            Some(0),
            "the same beat is a boundary again, because the reading under it changed"
        );
    }

    /// Verifies that lanes targeting parameters correctly construct `Operation::WriteParam`.
    #[test]
    fn a_parameter_lane_emits_a_parameter_write() {
        let lane = Lane::new(
            LaneTarget::Param {
                deck: 1,
                param: ParamAt {
                    node: None,
                    key: "twist".to_string(),
                },
            },
            1.0,
            0.0,
        );
        assert_eq!(
            lane.operation_at(0, StepMode::Sixteenth),
            Operation::WriteParam {
                deck: 1,
                param: ParamAt {
                    node: None,
                    key: "twist".to_string()
                },
                value: ParamValue::Scalar(0.0),
            }
        );
    }

    /// Verifies that muted lanes release fader holding to allow manual overrides.
    #[test]
    fn a_muted_lane_holds_no_fader() {
        let mut pattern = Pattern::empty();
        pattern.push(fader(0));
        assert_eq!(pattern.holder_of_fader(0), Some(0));
        assert_eq!(pattern.holder_of_fader(1), None, "no lane drives deck B");
        pattern.lane_mut(0).unwrap().set_muted(true);
        assert_eq!(
            pattern.holder_of_fader(0),
            None,
            "a muted lane drives nothing, so there is nothing for a fade to collide with"
        );
    }

    /// Verifies bank selection, bounds enforcement, and allocation of empty banks.
    #[test]
    fn four_banks_and_the_plus_is_the_next_empty_one() {
        let mut banks = Banks::default();
        assert_eq!(banks.armed(), 0);
        assert_eq!(banks.first_empty(), Some(0), "every bank starts empty");
        for bank in 0..BANKS {
            banks.at_mut(bank).unwrap().push(fader(0));
        }
        assert_eq!(
            banks.first_empty(),
            None,
            "with all four full the `+` has nowhere to land, and rule 04 asks the surface to say so"
        );
        assert!(banks.select(3));
        assert_eq!(banks.armed(), 3);
        assert!(!banks.select(BANKS), "there is no fifth bank");
        assert_eq!(banks.armed(), 3, "and a refused press moves nothing");
    }
}
