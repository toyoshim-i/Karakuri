//! **What a sequencer pattern is**, and nothing about where one is kept.
//!
//! A lane is a **fifth route** into `karakuri_operation` rather than a binding
//! ([ADR-0222](../../../docs/adr/0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md)):
//! it emits operations on the beat the way the pointer, the keys, a map and a
//! model emit them, so a lane needs no new operation to drive anything. What
//! was missing was the *authored state* — a pattern, which nothing in this
//! workspace held — and this crate is that state and only that.
//!
//! # What is here
//!
//! [`Pattern`] is **one bar**: a [`StepMode`] and a list of [`Lane`]s. A lane
//! is what it drives, sixteen slots, two levels and whether it is muted.
//! [`Banks`] is the session's four of them and which one is armed. [`Playhead`]
//! is the poll — the step index, remembered, so a caller can tell a boundary
//! from a frame.
//!
//! Every one of those is
//! [ADR-0320](../../../docs/adr/0320-a-pattern-is-one-bar-of-sixteen-slots-a-lane-is-a-target-and-two-levels-and-a-cell-is-a-bit.md)'s,
//! and the two that are not are named there: what a lane's target is spelled as
//! is [ADR-0321](../../../docs/adr/0321-a-lanes-target-is-an-operation-with-its-value-elided.md)
//! and lives in the vocabulary, and where the producer runs is
//! [ADR-0322](../../../docs/adr/0322-the-sequencer-is-polled-like-a-transition-live-only-and-its-writes-are-its-record.md)
//! and lives in whoever owns the frame loop.
//!
//! # What is deliberately not here
//!
//! **No serialiser, and no path.**
//! [ADR-0227](../../../docs/adr/0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md)
//! settles that a pattern is library data in two tiers on the arrangement's
//! shape — a name the operator typed, one path component, a place of its own
//! under the store root, bytes the store does not parse — and leaves the file
//! form open *"for the record that has something to serialise."* Nothing here
//! serialises: the console draws a pattern, the poll reads one, and neither
//! writes a file. `serde` and a `patterns/` directory arrive with the row that
//! saves one, which is the record that will have something to serialise.
//!
//! **No clock.** A step index is `floor(beats × steps_per_beat) mod count` — a
//! pure function of `Oscillator::beats`, handed in — so nothing in this crate
//! reads a clock and nothing in it derives one
//! ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
//!
//! **No engine and no surface.** A lane hands back an [`Operation`] and does
//! not apply it, which is the same seam `karakuri-console` sits on
//! ([ADR-0156](../../../docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)):
//! a pattern is not a drawing, and the write is checked where it lands.

use karakuri_operation::{LaneTarget, Operation, StepMode};

/// **How many slots a pattern stores, in both modes.**
///
/// Sixteen either way, and an eighth reads slot `2k`
/// ([`StepMode::slot_of`]), so a mode press changes a *reading* and never the
/// pattern: the finer mode and back returns exactly what was there. Sizing the
/// store to the count instead would throw eight steps away on one press with
/// nothing to confirm against, which is the alternative ADR-0320 refuses and
/// which rule 04 forbids.
pub const SLOTS: usize = 16;

/// **How many banks a session holds**: four, fixed, on the deck's precedent.
///
/// A bank list that grows is a control nobody drew, so the mock's `+` means
/// *the next empty one* and is disabled with a reason when all four are full
/// ([`Banks::first_empty`]). **A bank is not a saved name** — it is a position
/// in the session, the way a deck slot is, and a name is what a save files a
/// pattern under.
pub const BANKS: usize = 4;

/// **One lane**: what it drives, sixteen slots, the two levels a step is worth,
/// and whether it is muted.
///
/// # A cell is a bit and the levels are the lane's
///
/// The console's own two open questions were *"what an on cell is worth is
/// open"* and *"an on cell here means writing something and which something is
/// undecided"*, and the answer is that the level is not the cell's at all: a
/// lane on a fader with `on = 1.0, off = 0.0` is a gate, and the same lane with
/// `0.8` and `0.5` is a pulse. Two numbers, and they live where the target
/// lives because a level only means anything against what it is written to.
///
/// **An off step writes `off`; it does not write nothing.** A lane that wrote
/// only its on-steps would leave its target wherever the last on-step put it,
/// which makes a pattern a set of impulses — and the mock's lane A, nearly full
/// with two gaps, reads as a gate.
///
/// # Where the two levels come from
///
/// The surface that adds the lane fills them in: a fader's are 1.0 and 0.0, and
/// a parameter's are the bottom and top of the range **published to that
/// surface at the moment of the press** (ADR-0286), taken there rather than
/// read back later because a pattern outlives the Set it was written against.
/// The write is still checked where it lands (ADR-0223), which is P-0090: the
/// surface offers two numbers and does not decide whether they are allowed.
#[derive(Debug, Clone, PartialEq)]
pub struct Lane {
    target: LaneTarget,
    /// **Bit `k` is slot `k`.** `u16` and not `[bool; 16]`: sixteen bits is the
    /// whole bar, and a pattern is a value a surface holds per frame.
    steps: u16,
    on: f32,
    off: f32,
    muted: bool,
}

impl Lane {
    /// **A lane over `target`, writing `on` and `off`, with every slot off and
    /// nothing muted.**
    ///
    /// There is no constructor that takes the steps, and that is the drawing
    /// rather than caution: a lane arrives from `+ lane` with what it drives
    /// and nothing else — *"a lane with nothing to point at emits nothing, so
    /// there is no moment at which a lane exists and its target does not"* —
    /// and every step after that is one press of [`Operation::SetStep`].
    pub fn new(target: LaneTarget, on: f32, off: f32) -> Lane {
        Lane {
            target,
            steps: 0,
            on,
            off,
            muted: false,
        }
    }

    /// What this lane drives.
    pub fn target(&self) -> &LaneTarget {
        &self.target
    }

    /// What an on step writes.
    pub fn on(&self) -> f32 {
        self.on
    }

    /// What an off step writes.
    pub fn off(&self) -> f32 {
        self.off
    }

    /// **Whether the pattern is kept and drives nothing.**
    pub fn muted(&self) -> bool {
        self.muted
    }

    /// **A state and never a flip**, which is `Operation::SetLaneMute`'s own
    /// rule: a surface that could only flip has no way to arrive.
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    /// **Whether stored slot `slot` is on**, and `false` for a slot past the
    /// sixteen rather than a panic: a slot index arrives from a surface, and a
    /// surface offers rather than decides.
    pub fn slot_on(&self, slot: usize) -> bool {
        slot < SLOTS && self.steps & (1 << slot) != 0
    }

    /// **Set stored slot `slot`**, a state and never a flip. A slot past the
    /// sixteen writes nothing.
    pub fn set_slot(&mut self, slot: usize, on: bool) {
        if slot >= SLOTS {
            return;
        }
        match on {
            true => self.steps |= 1 << slot,
            false => self.steps &= !(1 << slot),
        }
    }

    /// **Whether step `step` of `mode` is on**, which is the slot that mode
    /// reads it off.
    pub fn step_on(&self, step: usize, mode: StepMode) -> bool {
        self.slot_on(mode.slot_of(step))
    }

    /// **What this lane writes at step `step`**: `on` where the step is on and
    /// `off` where it is not.
    pub fn value_at(&self, step: usize, mode: StepMode) -> f32 {
        match self.step_on(step, mode) {
            true => self.on,
            false => self.off,
        }
    }

    /// **The operation this lane emits at step `step`**, which is the whole of
    /// what a lane does: an address of the vocabulary plus a level (ADR-0321).
    ///
    /// It does not ask whether the lane is muted — [`Pattern::due`] does, so
    /// that *a muted lane emits nothing* is decided in one place.
    pub fn operation_at(&self, step: usize, mode: StepMode) -> Operation {
        self.target.operation(self.value_at(step, mode))
    }
}

/// **One pattern: one bar.**
///
/// The bar is fixed and the count follows the mode, so there is no length here
/// and no step count either — *"a length nobody sets leaves a count and a
/// length with nothing to say"* (ADR-0306).
///
/// **The mode belongs to the pattern**, which is the console's own sentence
/// about the pill that draws it: *"It is armed because it is what the pattern
/// is rather than a preference the head is holding."*
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pattern {
    mode: StepMode,
    lanes: Vec<Lane>,
}

impl Pattern {
    /// **An empty bank**: a sixteenth, and no lanes.
    ///
    /// The mock's `+` says what this draws as — *"the rows below are the empty
    /// pattern's, which is no rows at all: a lane arrives with what it drives,
    /// from the foot, so an empty pattern has nothing to draw and nothing to
    /// mute."*
    pub fn empty() -> Pattern {
        Pattern::default()
    }

    /// What one step of this pattern is worth.
    pub fn mode(&self) -> StepMode {
        self.mode
    }

    /// **Choose what a step is worth**, which changes a reading and not the
    /// pattern: the slots are sixteen either way ([`SLOTS`]).
    pub fn set_mode(&mut self, mode: StepMode) {
        self.mode = mode;
    }

    /// The lanes, in the order the rows are drawn.
    pub fn lanes(&self) -> &[Lane] {
        &self.lanes
    }

    /// One lane, or `None` for an index no row draws.
    pub fn lane_mut(&mut self, at: usize) -> Option<&mut Lane> {
        self.lanes.get_mut(at)
    }

    /// **Add a lane**, which is `Operation::PointLane` — the one control this
    /// bay draws for making one, and the reason there is no index in that
    /// payload.
    pub fn push(&mut self, lane: Lane) {
        self.lanes.push(lane);
    }

    /// Whether this bank has anything in it, which is what the mock's `+`
    /// lands on.
    pub fn is_empty(&self) -> bool {
        self.lanes.is_empty()
    }

    /// **Which step of this pattern `beats` is in**, which is ADR-0222's own
    /// formula: `floor(beats × steps_per_beat) mod count`, a pure function of
    /// `Oscillator::beats` and of nothing else.
    ///
    /// **Negative beats wrap forward rather than panicking.** An oscillator
    /// counts from the start of a session and does not go back, so this is a
    /// guard against a caller and not a case: `rem_euclid` on the floor keeps
    /// the answer inside `0..count` whatever is handed in.
    pub fn step_at(&self, beats: f64) -> usize {
        let count = self.mode.count() as f64;
        let step = (beats * self.mode.steps_per_beat())
            .floor()
            .rem_euclid(count);
        step as usize
    }

    /// **Everything this pattern asks for at step `step`**, in the order the
    /// rows are drawn — one operation per unmuted lane.
    ///
    /// **A muted lane emits nothing**, and this is the one place that is
    /// decided: the pattern is kept and drives nothing, which is what the
    /// mute is for.
    pub fn due(&self, step: usize) -> impl Iterator<Item = Operation> + '_ {
        let mode = self.mode;
        self.lanes
            .iter()
            .filter(|lane| !lane.muted())
            .map(move |lane| lane.operation_at(step, mode))
    }

    /// **Which lane is driving `deck`'s channel fader**, or `None`.
    ///
    /// The lookup
    /// [ADR-0323](../../../docs/adr/0323-a-scheduled-move-is-refused-on-a-control-a-lane-holds.md)
    /// is built on: a scheduled move on a control a lane holds is refused, and
    /// the refusal names the lane. **A muted lane holds nothing** — it drives
    /// nothing, so there is nothing for a fade to collide with, and muting is
    /// how an operator gets their fade back.
    ///
    /// It answers an index rather than a lane so that a refusal can say
    /// *lane 2* the way the console's labels count.
    pub fn holder_of_fader(&self, deck: u8) -> Option<usize> {
        self.lanes.iter().position(|lane| {
            !lane.muted() && matches!(lane.target(), LaneTarget::Fader { deck: at } if *at == deck)
        })
    }
}

/// **The session's four banks, and which one the rows are reading.**
///
/// A bank is a position and a name is a file, and they are not the same thing
/// (ADR-0320) — so this holds no names, and nothing here saves or loads.
#[derive(Debug, Clone, PartialEq)]
pub struct Banks {
    banks: [Pattern; BANKS],
    armed: usize,
}

impl Default for Banks {
    /// **Four empty banks, with the first armed.**
    fn default() -> Banks {
        Banks {
            banks: Default::default(),
            armed: 0,
        }
    }
}

impl Banks {
    /// Which bank the rows are reading.
    pub fn armed(&self) -> usize {
        self.armed
    }

    /// **Choose which pattern the sequencer plays.** A bank past the four is
    /// refused by being ignored, which is the surface offering an index this
    /// type does not have.
    pub fn select(&mut self, bank: usize) -> bool {
        if bank >= BANKS {
            return false;
        }
        self.armed = bank;
        true
    }

    /// The armed pattern, which is what the rows draw and what the poll reads.
    pub fn pattern(&self) -> &Pattern {
        &self.banks[self.armed]
    }

    /// One bank by index, or `None` past the four.
    pub fn at(&self, bank: usize) -> Option<&Pattern> {
        self.banks.get(bank)
    }

    /// One bank by index, or `None` past the four.
    pub fn at_mut(&mut self, bank: usize) -> Option<&mut Pattern> {
        self.banks.get_mut(bank)
    }

    /// **The next bank with nothing in it**, which is what the mock's `+`
    /// means — and `None` when all four are full, which is where rule 04 asks
    /// the surface to say why the press cannot be used.
    pub fn first_empty(&self) -> Option<usize> {
        self.banks.iter().position(Pattern::is_empty)
    }
}

/// **The poll**: where the playhead was last, so a caller can tell a step
/// boundary from a frame.
///
/// # It is polled, and it is not a fourth clock
///
/// The producer runs on the render thread, once a frame, against `beats` —
/// which is what a transition already is one row finer
/// (`Transition::value_at(beats)`, `Selection::due(beats)`), and the engine has
/// no beat callback for it to be anything else (ADR-0322). Nothing here reads a
/// clock: `beats` is handed in, and it is a pure function of the `tick` records
/// and the tempo corrections.
///
/// # A skipped step is dropped, not caught up
///
/// If a stall carries a frame past two boundaries this answers the *current*
/// step and not both. The ir-spec's own sentence is the argument — *"a step
/// onset is seen at the next frame, which is right for a sequencer that writes
/// values and would not be for one that fires events"* — and emitting the
/// skipped one would put a value in the stream that is overwritten in the same
/// frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Playhead {
    last: Option<usize>,
}

impl Playhead {
    /// **The step that has just become current**, or `None` while the index
    /// has not moved.
    ///
    /// So a lane emits at a boundary and never twice for one step, which is
    /// what keeps a pattern's cost the 34 records a second ADR-0322 measures
    /// rather than one per frame.
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

    /// **Where the playhead is**, for a readout — `None` before the first
    /// frame, which is a bay that has not been polled rather than a step zero.
    pub fn at(&self) -> Option<usize> {
        self.last
    }

    /// **Forget where it was**, so the next poll emits whatever the step is
    /// now.
    ///
    /// Called where the *meaning* of an index changes rather than the index:
    /// a mode press re-reads the same bar at a different width and a bank
    /// press swaps the lanes underneath it, so a remembered index would hold
    /// the new reading silent until the bar came round.
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

    /// **ADR-0222's formula, at the boundaries it is about.** A sixteenth is a
    /// quarter of a beat, so the bar's sixteen steps land at every 0.25 and the
    /// index wraps at the bar rather than counting on with the session.
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

    /// **An eighth reads slot `2k`**, which is what makes a mode press a change
    /// of reading: the same sixteen slots, read every second one.
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

    /// **A mode press changes a reading and not the pattern**: the finer mode
    /// and back returns exactly what was there, which is the whole reason the
    /// store is sixteen slots in both modes (ADR-0320).
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

    /// **An off step writes the off level and not nothing**, which is what
    /// makes a lane on a fader a gate rather than a set of impulses.
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

    /// **A muted lane emits nothing**, and the pattern is kept: the mute is
    /// what a hand takes a lane back with (ADR-0322).
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

    /// **The poll emits exactly at a boundary and never twice for one step**,
    /// which is what keeps a four-lane pattern's cost 34 records a second
    /// rather than one per lane per frame.
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

    /// **A skipped step is dropped and not caught up.** A stall across two
    /// boundaries answers the current step once, because emitting the one it
    /// missed would put a value in the stream that is overwritten in the same
    /// frame.
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

    /// **A mode or a bank press forgets where the playhead was**, because the
    /// index it remembered means something else now.
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

    /// **A lane's target is an operation with its value elided**, and the
    /// fourth lane the console draws is the one that proves it reaches further
    /// than a slot number.
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

    /// **Which lane holds a fader**, which is the lookup a refused schedule
    /// names its lane from (ADR-0323) — and a muted lane holds nothing,
    /// because muting is how an operator gets their fade back.
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

    /// **Four banks, and the `+` is the next empty one.**
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
