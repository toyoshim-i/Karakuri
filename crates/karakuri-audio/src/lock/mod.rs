//! Phase-locked loop for synchronizing the local oscillator with audio beat estimates.
//!
//! Evaluates incoming [`Estimate`] values and produces [`Correction`] recommendations
//! (tempo adjustment and phase shift) for [`karakuri_signal::Oscillator`].
//!
//! ## Design & Invariants
//!
//! - **Oscillator Decoupling**: The engine renders exclusively from the local oscillator;
//!   corrections trim or acquire tempo without overriding clock autonomy.
//! - **Stiff Phase Lock**: Once locked, minor phase drift is gently trimmed via [`TRIM_TAU_PHASE`];
//!   large discrepancies require sustained evidence ([`RELOCK_EVIDENCE`]) before re-locking.
//! - **Feed-Forward Delay Compensation**: Computes overall lead (`ahead = analysis_lag + output_lag`)
//!   to ensure visual beats align precisely with audience acoustic perception.
//! - **Confidence Gating**: Estimates below [`GATE_CONFIDENCE`] produce no corrections, allowing
//!   the oscillator to free-run seamlessly during audio dropouts.

use karakuri_signal::Oscillator;

use crate::tempo::{Estimate, BPM_RANGE};

/// Minimum estimate confidence required to update phase or tempo.
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

/// Recommended phase and tempo adjustment emitted by [`BeatLock`].
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
    /// A performer moved the grid an octave.
    Octave,
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

    /// Evaluates the current estimate against the oscillator and returns a [`Correction`] if needed.
    ///
    /// `ahead` specifies the feed-forward lead time (analysis latency plus output display lag).
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
                        // Disagreeing estimate without sufficient evidence leaves the grid untouched.
                        None
                    }
                }
            }
        }
    }

    /// Records a manual tap, adjusting phase immediately and updating tempo on repeated taps.
    ///
    /// Leads by `output_lag` to align visual presentation with acoustic beats.
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

    /// Shifts the tracking grid by an octave factor (e.g. 2.0 for ×2, 0.5 for ÷2).
    ///
    /// Preserves current beat phase while relocating the tracking octave window.
    /// Resets accumulated agreement and disagreement evidence. Returns `None` if
    /// the target BPM falls outside [`crate::tempo::BPM_RANGE`].
    pub fn octave(&mut self, factor: f32, oscillator: &Oscillator) -> Option<Correction> {
        let bpm = oscillator.bpm() * factor;
        if !BPM_RANGE.contains(&bpm) {
            return None;
        }
        self.state = State::Locked;
        self.agreement = 0;
        self.disagreement = 0;
        self.candidate_bpm = bpm;
        self.error = 0.0;
        self.reason = Some(Reason::Octave);
        Some(Correction {
            bpm,
            shift: 0.0,
            // A person meant it, exactly as with a tap.
            confidence: 1.0,
        })
    }

    /// Updates the target tempo for free-running operation without forcing state changes.
    ///
    /// Resets accumulated agreement/disagreement evidence and zeros the displayed error.
    pub fn retarget(&mut self, bpm: f32) {
        if !bpm.is_finite() || bpm <= 0.0 {
            return;
        }
        self.agreement = 0;
        self.disagreement = 0;
        self.candidate_bpm = bpm;
        // Reset displayed phase error measured against previous tempo.
        self.error = 0.0;
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

/// Normalizes a beat phase difference into the range `[-0.5, 0.5)`.
///
/// An exact half-beat difference (0.5) wraps to -0.5 to provide consistent pull-back behavior.
pub fn wrap_beats(x: f32) -> f32 {
    if !x.is_finite() {
        return 0.0;
    }
    x - x.round()
}

#[cfg(test)]
mod tests;
