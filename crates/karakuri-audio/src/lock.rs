//! The beat lock: what the estimate is allowed to do to the local oscillator.
//!
//! The invariant this is written under is `README.md`'s — *rendering reads only
//! the local oscillator, never an external clock*. Nothing here becomes the
//! clock. It produces a [`Correction`]: a tempo and a phase shift, handed to
//! `Oscillator::correct`, recorded as a `tempo` record, and read back verbatim
//! on replay. The analyser never runs twice on the same session, which matters
//! because the analyser is allowed to improve and a session recorded today has
//! to replay the same way after it does.
//!
//! ## A grid that is predicted, not chased
//!
//! The tempo is stable for essentially all of a set, and steps a few times an
//! hour when one track replaces another. So the loop is deliberately **stiff**:
//! once locked, the oscillator runs the grid on its own and the estimate is
//! allowed only a slow trim ([`TRIM_TAU_PHASE`]) against small errors. It is
//! not asked to find each beat. A soft loop tuned to follow a wandering tempo
//! would be jittery in the state that matters in exchange for agility in a
//! transient nobody is judging — during a blend there are two tempi in the room
//! and no correct answer anyway.
//!
//! Distinguishing **"one estimate disagreed"** from **"the tempo has changed"**
//! is where this kind of system usually goes wrong, so it is explicit:
//!
//! - A disagreeing estimate moves the grid *not at all*.
//! - Disagreements are counted per **revision** — one per re-measurement,
//!   about four a second — and they have to agree *with each other* to count.
//!   [`RELOCK_EVIDENCE`] of them, roughly two seconds of a consistent new
//!   opinion, before the grid is re-acquired.
//! - Being a beat slow to notice a real change costs nothing anyone will see,
//!   because the change is already inside a transition. One spurious
//!   re-acquire costs a visibly wrong bar.
//!
//! ## The lead, and why a loop that merely tracks is visibly late
//!
//! Two delays sit either side of the correction and they do not cancel:
//!
//! - **A, analysis lag.** A beat in the room at wall time `T` cannot be
//!   measured until the buffer holding it has been delivered and the window
//!   covering it is complete: `A = input latency + half an analysis window`.
//!   `crate::device` publishes it.
//! - **D, output lag.** A frame prepared at `P` is not light until `P + D`:
//!   render, queue depth, present, and the display's own pipeline.
//!
//! A loop that drives the oscillator's phase *now* to the music as it was at
//! `now − A`, and then shows it `D` later, is late by `A + D` — consistently,
//! which reads as wrong rather than as jitter. So the target is not the
//! estimate's phase but the grid's phase **`A + D` further on**:
//!
//! > at wall time `P`, the oscillator's phase should be the phase the music
//! > will have at `P + D`, given an estimate describing the music at `P − A`.
//!
//! That is [`BeatLock::update`]'s `ahead` argument, and it is feed-forward: it
//! is computed from durations, not tuned against the error it removes. The
//! caller sums it, because the caller is the only one that knows how old the
//! estimate is by now.
//!
//! ## Confidence gates everything
//!
//! Below [`GATE_CONFIDENCE`] nothing happens at all — no trim, no evidence, no
//! re-acquire. An interface unplugged mid-set leaves a grid running at the
//! tempo it had, which is the only acceptable behaviour: the picture keeps its
//! tempo rather than stopping or lurching.

use karakuri_signal::Oscillator;

use crate::tempo::Estimate;

/// Below this, an estimate is not evidence of anything.
pub const GATE_CONFIDENCE: f32 = 0.35;

/// Consecutive agreeing revisions before a free-running grid locks. Three is
/// about three quarters of a second — long enough that a single lucky window
/// does not acquire, short enough that a set does not start unlocked.
pub const ACQUIRE_EVIDENCE: u32 = 3;

/// Consecutive *consistent* disagreeing revisions before a locked grid is
/// re-acquired. Eight is about two seconds. See the module doc for why this is
/// deliberately slow.
pub const RELOCK_EVIDENCE: u32 = 8;

/// How far apart two tempi can be and still be the same opinion.
pub const AGREE_BPM_RATIO: f32 = 0.015;

/// Phase error, in beats, beyond which the grid is not trimmed but *doubted*.
/// A small error is drift and is trimmed away; a large one is either a wrong
/// estimate or a changed track, and neither is fixed by pulling.
pub const TRIM_LIMIT: f32 = 0.08;

/// Time constant of the phase trim, in seconds. Slow on purpose: this exists to
/// take out the drift between the audio device's clock and the session's, not
/// to chase beats.
pub const TRIM_TAU_PHASE: f32 = 2.0;

/// Time constant of the tempo trim, in seconds. Slower still — a wrong tempo
/// compounds, so it is the last thing that should move on thin evidence.
pub const TRIM_TAU_TEMPO: f32 = 12.0;

/// How many taps are remembered, and how long a tap stays relevant.
const TAP_MEMORY: usize = 4;
const TAP_TIMEOUT_SECONDS: f64 = 3.0;

/// What the oscillator is told this frame.
///
/// Field for field a `tempo` record: live, this is computed and emitted; on
/// replay it is decoded and applied, and the oscillator cannot tell which
/// happened.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Correction {
    /// The tempo from now on.
    pub bpm: f32,
    /// Phase shift in beats, positive meaning the next beat arrives sooner.
    pub shift: f32,
    /// How much the estimate behind this was believed. Carried so that a replay
    /// shows the operator what the live run showed.
    pub confidence: f32,
}

/// Why a correction happened — for the status line, not for the record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// A free-running grid locked on.
    Acquired,
    /// A locked grid was re-acquired after sustained disagreement.
    Reacquired,
    /// The slow trim.
    Trim,
    /// A performer tapped.
    Tap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Free,
    Locked,
}

/// The controller. One per session, alongside the tracker.
pub struct BeatLock {
    state: State,
    /// The revision last counted as evidence, so extrapolated estimates are not
    /// mistaken for new opinions.
    seen: u64,
    agreement: u32,
    disagreement: u32,
    /// What the run of agreeing (or disagreeing) estimates has been saying.
    candidate_bpm: f32,
    error: f32,
    reason: Option<Reason>,
    taps: [f64; TAP_MEMORY],
    tap_count: usize,
}

impl Default for BeatLock {
    fn default() -> BeatLock {
        BeatLock::new()
    }
}

impl BeatLock {
    pub fn new() -> BeatLock {
        BeatLock {
            state: State::Free,
            seen: u64::MAX,
            agreement: 0,
            disagreement: 0,
            candidate_bpm: 0.0,
            error: 0.0,
            reason: None,
            taps: [f64::NEG_INFINITY; TAP_MEMORY],
            tap_count: 0,
        }
    }

    /// Whether the grid is locked to something rather than free-running.
    pub fn locked(&self) -> bool {
        self.state == State::Locked
    }

    /// The phase error last seen, in beats, signed — what a performer needs to
    /// see to know whether the offset dial wants nudging. Positive means the
    /// music is ahead of the picture.
    pub fn error(&self) -> f32 {
        self.error
    }

    /// What the last correction was for.
    pub fn reason(&self) -> Option<Reason> {
        self.reason
    }

    /// One frame. Returns what to tell the oscillator, or `None` for "leave it
    /// alone", which is most frames.
    ///
    /// `ahead` is the lead: how far past the instant the estimate describes the
    /// correction should aim — the estimate's age by now, plus the output lag.
    /// See the module doc. `step` is how much simulation time this frame
    /// advances, which is what makes the trim rates frame-rate independent.
    pub fn update(
        &mut self,
        estimate: &Estimate,
        ahead: f32,
        oscillator: &Oscillator,
        step: f32,
    ) -> Option<Correction> {
        let fresh = estimate.revision != self.seen;
        self.seen = estimate.revision;
        self.reason = None;

        if estimate.confidence < GATE_CONFIDENCE || estimate.bpm <= 0.0 || estimate.bpm.is_nan() {
            // Nothing is known. Not a reason to move anything — and not a
            // reason to forget the lock either: a dropout keeps its tempo.
            if fresh {
                self.agreement = 0;
                self.disagreement = 0;
            }
            return None;
        }

        let target = estimate.phase_ahead(ahead);
        let error = wrap_beats(target - oscillator.beat_phase());
        self.error = error;

        match self.state {
            State::Free => {
                if fresh {
                    self.count(Counter::Agreement, estimate.bpm);
                }
                if self.agreement >= ACQUIRE_EVIDENCE {
                    self.agreement = 0;
                    self.state = State::Locked;
                    self.reason = Some(Reason::Acquired);
                    // Acquisition is allowed to jump: nothing was locked, so
                    // there is no grid to disturb.
                    Some(Correction {
                        bpm: estimate.bpm,
                        shift: error,
                        confidence: estimate.confidence,
                    })
                } else {
                    None
                }
            }
            State::Locked => {
                let tempo_agrees = (estimate.bpm - oscillator.bpm()).abs()
                    <= oscillator.bpm() * AGREE_BPM_RATIO.max(f32::EPSILON);
                if tempo_agrees && error.abs() <= TRIM_LIMIT {
                    if fresh {
                        self.disagreement = 0;
                        self.candidate_bpm = estimate.bpm;
                    }
                    self.reason = Some(Reason::Trim);
                    Some(Correction {
                        bpm: oscillator.bpm()
                            + (estimate.bpm - oscillator.bpm()) * rate(step, TRIM_TAU_TEMPO),
                        shift: error * rate(step, TRIM_TAU_PHASE),
                        confidence: estimate.confidence,
                    })
                } else {
                    if fresh {
                        self.count(Counter::Disagreement, estimate.bpm);
                    }
                    if self.disagreement >= RELOCK_EVIDENCE {
                        self.disagreement = 0;
                        self.reason = Some(Reason::Reacquired);
                        Some(Correction {
                            bpm: estimate.bpm,
                            shift: error,
                            confidence: estimate.confidence,
                        })
                    } else {
                        // **One estimate disagreeing moves the grid not at
                        // all.** This is the branch the design is about.
                        None
                    }
                }
            }
        }
    }

    /// A performer tapping the beat. Authoritative — a tap is an instruction,
    /// not evidence, so it is applied in full.
    ///
    /// One tap sets the phase; three or more consistent taps set the tempo as
    /// well. `at` is a wall-clock instant in seconds, `output_lag` is the same
    /// `D` [`BeatLock::update`] leads by: a performer taps in time with what
    /// they hear, and what they want is the *picture* on the beat, so the
    /// oscillator has to be `D` ahead of the tap rather than on it.
    pub fn tap(&mut self, at: f64, output_lag: f32, oscillator: &Oscillator) -> Correction {
        if self.tap_count > 0 && at - self.taps[self.tap_count - 1] > TAP_TIMEOUT_SECONDS {
            // A tap after a long gap starts a new count rather than averaging
            // against a tempo from earlier in the set.
            self.tap_count = 0;
        }
        if self.tap_count == TAP_MEMORY {
            self.taps.rotate_left(1);
            self.tap_count -= 1;
        }
        self.taps[self.tap_count] = at;
        self.tap_count += 1;

        let bpm = self.tapped_tempo().unwrap_or(oscillator.bpm());
        // The tap says the music had phase 0 at the instant of the tap; the
        // picture drawn at that instant is seen `output_lag` later, so the
        // oscillator has to be that far past the beat already.
        let target = (output_lag * bpm / 60.0).rem_euclid(1.0);
        let error = wrap_beats(target - oscillator.beat_phase());

        self.state = State::Locked;
        self.agreement = 0;
        self.disagreement = 0;
        self.error = error;
        self.reason = Some(Reason::Tap);
        Correction {
            bpm,
            shift: error,
            // A tap is as certain as anything gets here: a person meant it.
            confidence: 1.0,
        }
    }

    /// The tempo the taps imply, if there are enough of them and they agree.
    fn tapped_tempo(&self) -> Option<f32> {
        if self.tap_count < 3 {
            return None;
        }
        let intervals: Vec<f64> = self.taps[..self.tap_count]
            .windows(2)
            .map(|w| w[1] - w[0])
            .collect();
        let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
        if mean <= 0.0 {
            return None;
        }
        // Taps that disagree with each other are a performer finding the beat,
        // not stating it.
        if intervals.iter().any(|i| (i - mean).abs() > mean * 0.25) {
            return None;
        }
        Some((60.0 / mean) as f32)
    }

    /// Count one more opinion, or start a new run if it is a different one.
    fn count(&mut self, counter: Counter, bpm: f32) {
        let agrees = (bpm - self.candidate_bpm).abs() <= self.candidate_bpm * AGREE_BPM_RATIO;
        let n = match counter {
            Counter::Agreement => &mut self.agreement,
            Counter::Disagreement => &mut self.disagreement,
        };
        // A run of opinions has to be a run of the *same* opinion. Otherwise
        // four estimates disagreeing with the grid in four different directions
        // would add up to evidence for a tempo none of them named.
        *n = if agrees { *n + 1 } else { 1 };
        self.candidate_bpm = bpm;
    }
}

/// Which run of consistent opinions is being counted: the one that would lock
/// a free-running grid, or the one that would re-acquire a locked one.
#[derive(Clone, Copy)]
enum Counter {
    Agreement,
    Disagreement,
}

/// A first-order approach rate for a time constant, given this frame's step.
/// Frame-rate independent: two frames of one step and one frame of two steps
/// approach by the same amount.
fn rate(step: f32, tau: f32) -> f32 {
    if step <= 0.0 || tau <= 0.0 {
        return 0.0;
    }
    1.0 - (-step / tau).exp()
}

/// A phase difference into `[-0.5, 0.5)`.
///
/// **Exactly half a beat wraps to −0.5**, i.e. to "pull the grid back" rather
/// than "push it forward". The two are equally correct and the reason to pick
/// one is that the boundary must not flip: an error hovering at half a beat
/// would otherwise alternate between the two, and the grid would be yanked in
/// opposite directions on consecutive frames.
pub fn wrap_beats(x: f32) -> f32 {
    if !x.is_finite() {
        return 0.0;
    }
    x - x.round()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;
    /// Analysis lag: a device buffer plus half a window, near enough.
    const A: f32 = 0.032;
    /// Output lag: two frames of queue plus a display.
    const D: f32 = 0.045;
    const ESTIMATE_INTERVAL: f64 = 0.25;

    /// The music: phase 0 at wall time 0, running at `bpm`.
    fn music_phase(bpm: f32, at: f64) -> f32 {
        (at * bpm as f64 / 60.0).rem_euclid(1.0) as f32
    }

    struct Session {
        oscillator: Oscillator,
        lock: BeatLock,
        now: f64,
        published: f64,
        estimate: Estimate,
        revision: u64,
    }

    impl Session {
        fn new(free_running_bpm: f32) -> Session {
            Session {
                oscillator: Oscillator::new(free_running_bpm),
                lock: BeatLock::new(),
                now: 0.0,
                published: f64::NEG_INFINITY,
                estimate: Estimate::unknown(0.0),
                revision: 0,
            }
        }

        /// One frame: publish an estimate when one is due, correct, advance.
        /// `lead` is what makes the difference the whole design is about — with
        /// it, the correction aims at where the music will be when this frame
        /// is light; without it, at where the music was when it was measured.
        fn frame(&mut self, truth: Option<(f32, f32)>, lead: bool) {
            if self.now - self.published >= ESTIMATE_INTERVAL {
                self.published = self.now;
                self.revision += 1;
                self.estimate = match truth {
                    // The estimate describes the music as it was A ago.
                    Some((bpm, confidence)) => Estimate {
                        bpm,
                        phase: music_phase(bpm, self.now - A as f64),
                        confidence,
                        at: self.now - A as f64,
                        revision: self.revision,
                    },
                    None => Estimate {
                        revision: self.revision,
                        ..Estimate::unknown(self.now)
                    },
                };
            }

            let age = (self.now - self.published) as f32 + A;
            let ahead = if lead { age + D } else { 0.0 };
            if let Some(c) = self
                .lock
                .update(&self.estimate, ahead, &self.oscillator, DT)
            {
                self.oscillator.correct(c.bpm, c.shift);
            }
            self.oscillator.advance(1, DT);
            self.now += f64::from(DT);
        }

        fn run(&mut self, seconds: f32, truth: Option<(f32, f32)>, lead: bool) {
            for _ in 0..(seconds / DT) as u32 {
                self.frame(truth, lead);
            }
        }

        /// What the audience sees: the oscillator's phase as it will be when
        /// this frame is light, against the music at that same instant.
        fn visible_error(&self, bpm: f32) -> f32 {
            wrap_beats(self.oscillator.beat_phase() - music_phase(bpm, self.now + D as f64))
        }
    }

    /// **The assertion the whole change exists for.** A grid fed estimates that
    /// are `A` old, drawn `D` before it is seen, has to put the beat on the
    /// beat — and the same session without the lead has to be visibly late by
    /// exactly `A + D`, or this test would pass against a loop that ignored
    /// both.
    #[test]
    fn the_beat_lands_on_the_beat_including_the_analysis_and_output_lag() {
        let bpm = 128.0;

        let mut led = Session::new(120.0);
        led.run(12.0, Some((bpm, 0.9)), true);
        let error = led.visible_error(bpm);
        assert!(
            error.abs() < 0.01,
            "the visible beat is {error} beats out with the lead applied"
        );

        let mut naive = Session::new(120.0);
        naive.run(12.0, Some((bpm, 0.9)), false);
        let late = naive.visible_error(bpm);
        let expected = (A + D) * bpm / 60.0;
        assert!(
            (late.abs() - expected).abs() < 0.03,
            "without the lead the picture should be {expected} beats late; it was {late}"
        );
        // And that is a difference a person would see: an eighth of a beat at
        // 128 bpm is 60 ms.
        assert!(late.abs() > 0.1);
    }

    /// Locked means locked: after convergence the grid does not hunt.
    #[test]
    fn a_locked_grid_does_not_hunt() {
        let bpm = 128.0;
        let mut session = Session::new(120.0);
        session.run(10.0, Some((bpm, 0.9)), true);

        let mut worst: f32 = 0.0;
        let mut bpm_low = f32::INFINITY;
        let mut bpm_high: f32 = 0.0;
        for _ in 0..(4.0 / DT) as u32 {
            session.frame(Some((bpm, 0.9)), true);
            worst = worst.max(session.visible_error(bpm).abs());
            bpm_low = bpm_low.min(session.oscillator.bpm());
            bpm_high = bpm_high.max(session.oscillator.bpm());
        }
        assert!(worst < 0.02, "the grid wandered by {worst} beats");
        assert!(
            bpm_high - bpm_low < 0.5,
            "the tempo hunted between {bpm_low} and {bpm_high}"
        );
    }

    /// One estimate disagreeing must move the grid **not at all**. This is the
    /// failure mode that costs a visibly wrong bar.
    #[test]
    fn a_single_wrong_estimate_does_not_move_the_grid() {
        let bpm = 128.0;
        let mut session = Session::new(120.0);
        session.run(10.0, Some((bpm, 0.9)), true);
        let before = (session.oscillator.bpm(), session.oscillator.beats());

        // One revision claiming a wildly different tempo, at high confidence.
        session.published = f64::NEG_INFINITY;
        session.frame(Some((90.0, 0.95)), true);
        // ...and then straight back to the truth.
        session.run(1.0, Some((bpm, 0.9)), true);

        assert!(
            (session.oscillator.bpm() - before.0).abs() < 0.5,
            "one wrong estimate moved the tempo from {} to {}",
            before.0,
            session.oscillator.bpm()
        );
        assert!(
            session.visible_error(bpm).abs() < 0.02,
            "one wrong estimate moved the grid by {} beats",
            session.visible_error(bpm)
        );
    }

    /// A real tempo change: ignored at first, then re-acquired, and converged
    /// again. Drift during the transition is acceptable; being wrong afterwards
    /// is not.
    #[test]
    fn a_real_tempo_change_is_re_acquired_after_evidence_and_not_before() {
        let mut session = Session::new(120.0);
        session.run(10.0, Some((128.0, 0.9)), true);
        assert!(session.lock.locked());

        // The first second of the new tempo is evidence being gathered, and the
        // grid must still be running the old one.
        session.run(1.0, Some((140.0, 0.9)), true);
        assert!(
            (session.oscillator.bpm() - 128.0).abs() < 1.0,
            "the grid moved to {} before the evidence was in",
            session.oscillator.bpm()
        );

        // Given a few seconds it follows.
        session.run(8.0, Some((140.0, 0.9)), true);
        assert!(
            (session.oscillator.bpm() - 140.0).abs() < 1.0,
            "the grid never re-acquired: {}",
            session.oscillator.bpm()
        );
        assert!(
            session.visible_error(140.0).abs() < 0.02,
            "after re-acquiring, the beat is {} out",
            session.visible_error(140.0)
        );
    }

    /// The dropout: someone unplugs the interface. The grid keeps its tempo and
    /// keeps running, and picks up again without a lurch when it returns.
    #[test]
    fn a_dropout_keeps_the_tempo_and_the_return_does_not_lurch() {
        let bpm = 128.0;
        let mut session = Session::new(120.0);
        session.run(10.0, Some((bpm, 0.9)), true);
        let locked_bpm = session.oscillator.bpm();

        // Five seconds of nothing.
        session.run(5.0, None, true);
        assert_eq!(
            session.oscillator.bpm(),
            locked_bpm,
            "a dropout changed the tempo"
        );
        // It kept running: the phase is still where a grid at that tempo would
        // put it, which is what "keeps its tempo" has to mean.
        assert!(
            session.visible_error(bpm).abs() < 0.05,
            "the grid drifted {} beats through a dropout",
            session.visible_error(bpm)
        );

        // And when it comes back there is no jump.
        let before = session.oscillator.beats();
        session.run(1.0, Some((bpm, 0.9)), true);
        let ran = session.oscillator.beats() - before;
        assert!(
            (ran - bpm as f64 / 60.0).abs() < 0.1,
            "the return lurched by {} beats",
            ran - bpm as f64 / 60.0
        );
    }

    /// A weak estimate is not evidence: a grid with nothing to go on
    /// free-runs at whatever `--bpm` said, rather than being dragged around.
    #[test]
    fn a_weak_estimate_leaves_the_oscillator_free_running() {
        let mut session = Session::new(120.0);
        session.run(6.0, Some((140.0, GATE_CONFIDENCE - 0.01)), true);
        assert!(!session.lock.locked());
        assert_eq!(session.oscillator.bpm(), 120.0);
    }

    // -- tap ----------------------------------------------------------------

    /// Three taps at a steady rate set the tempo, and the picture lands on the
    /// tap rather than `D` after it.
    #[test]
    fn taps_set_the_tempo_and_lead_the_output_lag() {
        let mut oscillator = Oscillator::new(120.0);
        let mut lock = BeatLock::new();
        let interval = 0.5; // 120 bpm... let the taps say 150.
        let interval = interval * 120.0 / 150.0;

        let mut correction = None;
        for i in 0..4 {
            correction = Some(lock.tap(i as f64 * interval, D, &oscillator));
        }
        let correction = correction.expect("a tap always corrects");
        assert!(
            (correction.bpm - 150.0).abs() < 1.0,
            "four taps at 150 bpm read {}",
            correction.bpm
        );
        oscillator.correct(correction.bpm, correction.shift);

        // The oscillator is `D` past the beat at the instant of the tap, so
        // what is drawn now is on the beat when it is seen.
        let expected = (D * correction.bpm / 60.0).rem_euclid(1.0);
        assert!(
            wrap_beats(oscillator.beat_phase() - expected).abs() < 1e-3,
            "after a tap the phase is {} and should be {expected}",
            oscillator.beat_phase()
        );
        assert!(lock.locked());
    }

    /// One tap is a phase statement, not a tempo one — and so are two. Two
    /// taps are one interval, and one interval cannot be checked against
    /// anything: a performer's first two taps are them finding the beat, and a
    /// tempo taken from those would have to be un-tapped.
    #[test]
    fn fewer_than_three_taps_move_the_phase_and_leave_the_tempo_alone() {
        let oscillator = Oscillator::new(128.0);
        let mut lock = BeatLock::new();
        assert_eq!(lock.tap(4.0, 0.0, &oscillator).bpm, 128.0);
        // A second tap half a second later would be 120 bpm if two were enough.
        assert_eq!(lock.tap(4.5, 0.0, &oscillator).bpm, 128.0);
        // The third is what makes it a tempo.
        assert!((lock.tap(5.0, 0.0, &oscillator).bpm - 120.0).abs() < 1.0);
    }

    /// Taps a performer is still finding the beat with do not set a tempo.
    #[test]
    fn inconsistent_taps_do_not_set_a_tempo() {
        let oscillator = Oscillator::new(128.0);
        let mut lock = BeatLock::new();
        for at in [0.0, 0.5, 0.9, 1.8] {
            lock.tap(at, 0.0, &oscillator);
        }
        assert_eq!(
            lock.tap(2.0, 0.0, &oscillator).bpm,
            128.0,
            "taps that disagree with each other set a tempo"
        );
    }

    // -- the wrap -----------------------------------------------------------

    #[test]
    fn a_phase_error_wraps_the_short_way_and_half_a_beat_pulls_back() {
        assert_eq!(wrap_beats(0.1), 0.1);
        assert!((wrap_beats(0.9) + 0.1).abs() < 1e-6);
        assert!((wrap_beats(-0.9) - 0.1).abs() < 1e-6);
        // The boundary, decided rather than left to float: half a beat pulls
        // back. What matters is that it does not alternate.
        assert_eq!(wrap_beats(0.5), -0.5);
        assert_eq!(wrap_beats(0.5), wrap_beats(0.5));
        assert_eq!(wrap_beats(f32::NAN), 0.0);
    }

    /// The trim rate depends on elapsed simulation time and not on how it was
    /// grouped — the same rule every other rate in this repository follows.
    #[test]
    fn the_trim_rate_is_frame_rate_independent() {
        let one_big = rate(2.0 * DT, TRIM_TAU_PHASE);
        let two_small = 1.0 - (1.0 - rate(DT, TRIM_TAU_PHASE)).powi(2);
        assert!((one_big - two_small).abs() < 1e-6);
    }
}
