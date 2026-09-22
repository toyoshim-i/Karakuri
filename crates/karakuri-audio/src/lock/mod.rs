//! The beat lock: what the estimate is allowed to do to the local oscillator.
//!
//! The invariant this is written under is that *rendering reads only
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
//!
//! **Confidence says how well the grid fits the novelty and nothing else.** It
//! used to carry octave uncertainty as well, which meant a tempo read an octave
//! out arrived here looking like an absent one and the grid simply never
//! locked — a bug that read as silence. [`crate::tempo`] settles the octave by
//! folding now, so what reaches this gate is only ever "is there a beat and
//! does this grid sit on it".
//!
//! ## What a person can say that a measurement cannot
//!
//! Two controls here are **instructions rather than evidence**, and both are
//! applied in full and immediately: [`BeatLock::tap`], and [`BeatLock::octave`]
//! for the one thing the estimator cannot infer — which octave the operator
//! wants. See [`crate::tempo`] for why that decision is a person's.

use karakuri_signal::Oscillator;

use crate::tempo::{Estimate, BPM_RANGE};

/// Below this, an estimate is not evidence of anything.
///
/// It sits in a wide empty gap rather than on a slope, which is why it did not
/// have to move when [`crate::tempo`]'s confidence stopped carrying the octave.
/// Measured on synthesised material: broadband noise reads 0.01, a swell 0.01,
/// a sustained tone 0.18 — and everything with a pulse in it, including a grid
/// deliberately left an octave out, reads above 0.98. The gate separates "is
/// there a beat" from "there is not", and nothing lands in between.
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

    /// A performer naming the grid's tempo outright, rather than by a factor
    /// or by tapping it — `Operation::SetFreeRunTempo`, which is *what the grid
    /// runs at with nothing driving it*.
    ///
    /// # It moves nothing, which is why it hands back no [`Correction`]
    ///
    /// [`BeatLock::octave`] beside it computes a tempo and returns one, because
    /// the factor is all the operator gave it. Here the operator gave the
    /// number, and that number reaches the oscillator as a `tempo` record
    /// through `audio::apply_tempo` — the one road, live and on replay. What is
    /// left for this lock is the state the *record* does not carry, and this is
    /// it.
    ///
    /// # The run of evidence goes, for [`BeatLock::octave`]'s own reason
    ///
    /// `agreement` and `disagreement` count consecutive
    /// estimates saying one thing about the grid that was there. The grid has
    /// moved, and the tracker goes on publishing from the window it had for up
    /// to one `ESTIMATE_INTERVAL` afterwards — so a [`RELOCK_EVIDENCE`] run
    /// part-served by opinions formed before the press would take the grid back
    /// in less than the two seconds that number is built on, and the operator
    /// would watch the control undo itself. Cleared, a room that really is at
    /// another tempo pays a whole fresh run for it: the tracker deciding,
    /// rather than a count left over from before anybody pressed anything.
    ///
    /// `candidate_bpm` becomes what was named, so the first
    /// estimate after the press starts a run of its own rather than continuing
    /// one about a tempo nobody is asking for.
    ///
    /// # The state is left exactly as it is, and that is the one place this
    /// parts company with a tap and an octave
    ///
    /// Both of those set `State::Locked`, and both are a performer saying
    /// what the grid is locked *to*: a tap is the room's beat, and an octave is
    /// a tracked grid being corrected. This says what the grid **runs at**, and
    /// says nothing about whether anything is driving it — so the lock goes on
    /// answering that question from what it has measured:
    ///
    /// - **Not `Locked`.** [`BeatLock::locked`] is a readout — it is drawn on
    ///   the panel every frame and printed — and *locked* about a room with no
    ///   beat in it is the confident wrong judgement
    ///   `docs/principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md`
    ///   refuses. It would also cost the operator the thing they set the tempo
    ///   *for*: a beat arriving afterwards would need [`RELOCK_EVIDENCE`]'s
    ///   eight revisions to take a grid that [`ACQUIRE_EVIDENCE`]'s three would
    ///   have taken from a free-running one, so naming a rough tempo before the
    ///   music started would make the music slower to lock than saying nothing.
    /// - **Not `Free` either.** A room that *is* being tracked stays tracked:
    ///   dropping to `Free` would let the next three agreeing estimates
    ///   **jump** the grid — acquisition is allowed to jump, because nothing
    ///   was locked — where a locked grid trims towards them over
    ///   [`TRIM_TAU_TEMPO`].
    ///
    /// # No band and no range, and that is not an omission
    ///
    /// What a tempo may be is not this lock's to say — [`BPM_RANGE`] says of
    /// itself that it is not the range of answers, and a grid at 240 is a
    /// perfectly good grid. The ±15% a *hand* is held to is a guard against a
    /// mis-click and lives where the press becomes an operation, which is the
    /// console; a band written here would sit in the path every estimate
    /// travels, and a grid that cannot follow the music is a worse failure than
    /// a hand that can ask for anything
    /// ([ADR-0291](../../../docs/adr/0291-the-tempo-figure-is-the-track-and-the-band-is-a-guard-on-the-hand.md)).
    ///
    /// What is refused is a number that is not a tempo at all: `candidate_bpm`
    /// is compared against on every estimate and a NaN compares false with
    /// everything, so a run could never be counted again.
    pub fn retarget(&mut self, bpm: f32) {
        if !bpm.is_finite() || bpm <= 0.0 {
            return;
        }
        self.agreement = 0;
        self.disagreement = 0;
        self.candidate_bpm = bpm;
        // The phase does not move — [`BeatLock::octave`]'s sentence, and the
        // reason is the same one line along: `Oscillator::correct` takes a
        // shift of zero for this record, so the beat the performer can see
        // stays where it is. What is zeroed is the *reading*: the error on the
        // panel was measured against a tempo that is gone, and in a room below
        // `GATE_CONFIDENCE` nothing would ever overwrite it.
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
