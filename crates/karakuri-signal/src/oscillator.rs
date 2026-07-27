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
/// The only state is `t`, the elapsed simulation time (`sum(steps * dt)`
/// across every [`advance`](Oscillator::advance) call). Everything derived
/// from the oscillator — musical phase here, and every synthesized signal in
/// [`bus`](crate::bus) — is a function of `t` and `bpm` alone, deliberately:
/// two tick histories that reach the same elapsed time by a different route
/// (one big step vs. several small ones) are the same point in the session as
/// far as the oscillator is concerned. The IR spec's spawn accumulator agrees
/// on the quantity — `spawn_rate * dt * float(steps)` is what a frame adds,
/// however its steps were grouped — while differing on the grain: the engine
/// advances it once per substep, because a batch of new elements has to land
/// between two element passes rather than all of them before the first.
/// Nothing here depends on that distinction, but it is the one place the two
/// could be mistaken for saying the same thing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oscillator {
    bpm: f32,
    t: f64,
}

/// Tempo is clamped into this range. The lower bound matters: every noise
/// signal derives its phase from beats, so a tempo of zero would freeze all of
/// them at once while nothing reported an error — a failure that looks like a
/// broken generator rather than a bad tempo.
pub const BPM_RANGE: std::ops::RangeInclusive<f32> = 1.0..=1000.0;

impl Oscillator {
    /// A fresh oscillator at phase zero, ticking at `bpm`, clamped into
    /// [`BPM_RANGE`]. A NaN tempo becomes the low bound rather than poisoning
    /// every phase downstream.
    pub fn new(bpm: f32) -> Oscillator {
        let bpm = if bpm.is_nan() {
            *BPM_RANGE.start()
        } else {
            bpm.clamp(*BPM_RANGE.start(), *BPM_RANGE.end())
        };
        Oscillator { bpm, t: 0.0 }
    }

    /// Advance by `steps` simulation steps of `dt` seconds each.
    ///
    /// This mirrors a `tick` record exactly: `steps` is the field the engine
    /// emits (from real time, live; read back verbatim on replay), and `dt` is
    /// the fixed simulation step from the Set. Nothing here reads a clock —
    /// both arguments are handed in.
    pub fn advance(&mut self, steps: u8, dt: f32) {
        self.t += steps as f64 * dt as f64;
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
    fn one_step_of_double_dt_reaches_the_same_state_as_two_steps_of_dt() {
        // Same total elapsed time, different tick history — and, by design,
        // the same resulting phase: `t` (and everything derived from it) only
        // ever sees the product `steps * dt`, never the call count. See the
        // struct docs, and `bus::tests` for the same property carried through
        // to the synthesized signals.
        let mut merged = Oscillator::new(90.0);
        merged.advance(1, 0.2);

        let mut split = Oscillator::new(90.0);
        split.advance(1, 0.1);
        split.advance(1, 0.1);

        assert!((merged.t() - split.t()).abs() < 1e-12);
        assert_eq!(merged.beat_phase(), split.beat_phase());
        assert_eq!(merged.bar_phase(), split.bar_phase());
    }
}

#[cfg(test)]
mod tempo_tests {
    use super::*;

    #[test]
    fn a_zero_or_negative_tempo_cannot_freeze_every_noise_signal() {
        // Noise phase is derived from beats, so a tempo of zero would stop all
        // of it at once and report nothing. Clamping turns a silent freeze into
        // a visibly wrong-but-running tempo.
        for bad in [0.0, -120.0, f32::NAN] {
            let osc = Oscillator::new(bad);
            assert!(osc.bpm() >= *BPM_RANGE.start(), "{bad} survived clamping");
        }
    }

    #[test]
    fn an_ordinary_tempo_is_untouched() {
        assert_eq!(Oscillator::new(128.0).bpm(), 128.0);
    }
}
