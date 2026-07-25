//! The local oscillator: the single source of truth for phase and tempo.
//!
//! Rendering reads this and never an external clock. External tempo input, when
//! it exists, is a correction applied here — so a dropped or jittering source
//! degrades the correction rather than the clock.
//!
//! **The oscillator advances by simulation steps, never by wall clock.**
//! [`Oscillator::advance`] takes exactly the two quantities a `tick` record
//! carries — a step count and a fixed `dt` — and nothing else. There is no
//! clock read anywhere in this module; see `tests/no_clock_access.rs` for a
//! standing check of that.

/// Beats per bar. v0.2 of the IR spec has no time-signature concept anywhere
/// (no `bind` field, no Set-file record), so this is a fixed assumption of
/// common time rather than something derived from the record stream. Treat it
/// as provisional until a real time signature shows up in the format.
pub const BEATS_PER_BAR: u32 = 4;

/// Phase and tempo, advanced by simulation steps rather than by wall time.
///
/// Two quantities accumulate across calls to [`advance`](Oscillator::advance),
/// and they are deliberately kept separate rather than folded into one:
///
/// - `t`, the elapsed simulation time (`sum(steps * dt)`). Musical phase —
///   [`beat_phase`](Oscillator::beat_phase) and [`bar_phase`](Oscillator::bar_phase)
///   — is defined in terms of `t`, because tempo is a real-time-domain concept:
///   a beat is a duration, and two histories that reach the same elapsed time
///   are at the same point in the bar regardless of how they got there.
/// - `step`, the elapsed step count (`sum(steps)`). This is the running total
///   of the `steps` field taken verbatim off the `tick` records that produced
///   it — the record stream itself, not a quantity derived from it. Anything
///   that must stay a pure function of *which* tick history occurred, not just
///   of how much time it summed to, is keyed off this instead of off `t`. The
///   noise signal in [`bus`](crate::bus) is the reason this field exists: two
///   step histories that reach the same `t` by a different route (one big
///   step vs. several small ones) are different record streams, and are
///   allowed — expected — to diverge downstream of that difference.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oscillator {
    bpm: f32,
    t: f64,
    step: u64,
}

impl Oscillator {
    /// A fresh oscillator at phase zero, ticking at `bpm`.
    pub fn new(bpm: f32) -> Oscillator {
        Oscillator {
            bpm,
            t: 0.0,
            step: 0,
        }
    }

    /// Advance by `steps` simulation steps of `dt` seconds each.
    ///
    /// This mirrors a `tick` record exactly: `steps` is the field the engine
    /// emits (from real time, live; read back verbatim on replay), and `dt` is
    /// the fixed simulation step from the Set. Nothing here reads a clock —
    /// both arguments are handed in.
    pub fn advance(&mut self, steps: u8, dt: f32) {
        self.t += steps as f64 * dt as f64;
        self.step += steps as u64;
    }

    /// The oscillator's tempo. The local oscillator is the single source of
    /// truth for it, so this is not corrected against anything else in V1.
    pub fn bpm(&self) -> f32 {
        self.bpm
    }

    /// Elapsed simulation time in seconds, `sum(steps * dt)` across every call
    /// to [`advance`](Oscillator::advance) so far. Never wall clock.
    pub fn t(&self) -> f64 {
        self.t
    }

    /// Elapsed step count, `sum(steps)` across every call to
    /// [`advance`](Oscillator::advance) so far — the running total of the
    /// `tick` records' `steps` field itself, distinct from `t`. See the struct
    /// docs for why this is kept separate.
    pub fn step_index(&self) -> u64 {
        self.step
    }

    /// Position within the current beat, `0.0..1.0`. `0.0` is the instant of
    /// the beat; the value rises linearly and wraps at the next beat.
    pub fn beat_phase(&self) -> f32 {
        let beats_per_second = self.bpm as f64 / 60.0;
        (self.t * beats_per_second).rem_euclid(1.0) as f32
    }

    /// Position within the current bar, `0.0..1.0`, using [`BEATS_PER_BAR`].
    pub fn bar_phase(&self) -> f32 {
        let beats_per_second = self.bpm as f64 / 60.0;
        let beats = self.t * beats_per_second;
        (beats / BEATS_PER_BAR as f64).rem_euclid(1.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_uses_only_its_arguments() {
        let mut a = Oscillator::new(120.0);
        let mut b = Oscillator::new(120.0);
        a.advance(1, 1.0 / 60.0);
        b.advance(1, 1.0 / 60.0);
        assert_eq!(a, b);
    }

    #[test]
    fn t_accumulates_steps_times_dt() {
        let mut osc = Oscillator::new(120.0);
        osc.advance(2, 0.5);
        osc.advance(1, 0.25);
        assert!((osc.t() - 1.25).abs() < 1e-12);
        assert_eq!(osc.step_index(), 3);
    }

    #[test]
    fn beat_phase_wraps_at_bpm() {
        // At 60 bpm, one beat is exactly one second.
        let mut osc = Oscillator::new(60.0);
        assert_eq!(osc.beat_phase(), 0.0);
        osc.advance(1, 0.5);
        assert!((osc.beat_phase() - 0.5).abs() < 1e-6);
        osc.advance(1, 0.5);
        assert!(osc.beat_phase().abs() < 1e-6);
    }

    #[test]
    fn bar_phase_wraps_after_beats_per_bar_beats() {
        let mut osc = Oscillator::new(60.0);
        osc.advance(1, (BEATS_PER_BAR as f32) - 0.5);
        assert!((osc.bar_phase() - 0.875).abs() < 1e-6);
        osc.advance(1, 0.5);
        assert!(osc.bar_phase().abs() < 1e-6);
    }

    #[test]
    fn one_step_of_double_dt_reaches_the_same_time_as_two_steps() {
        // Same elapsed time, different tick history: `t` agrees...
        let mut merged = Oscillator::new(90.0);
        merged.advance(1, 0.2);

        let mut split = Oscillator::new(90.0);
        split.advance(1, 0.1);
        split.advance(1, 0.1);

        assert!((merged.t() - split.t()).abs() < 1e-12);
        assert_eq!(merged.beat_phase(), split.beat_phase());

        // ...but `step` does not: it is the tick history itself, not a
        // function of the time it summed to.
        assert_ne!(merged.step_index(), split.step_index());
        assert_eq!(merged.step_index(), 1);
        assert_eq!(split.step_index(), 2);
    }
}
