//! The local oscillator: the single source of truth for phase and tempo.
//!
//! Advances strictly by simulation steps, never by reading external clocks.
//! External tempo or MIDI sources apply corrections via [`Oscillator::correct`],
//! adjusting tempo and phase shifts without mutating historical beat positions.
//!
//! ## Musical Position and Elapsed Beats
//!
//! - [`Oscillator::beats`]: Musical position including phase corrections.
//! - [`Oscillator::elapsed_beats`]: Accumulated tempo scaling excluding phase shifts,
//!   preventing noise generator lattice discontinuities during phase realignment.

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
    /// The simulation time the current tempo took effect at. Zero until the
    /// first correction, which is what makes an uncorrected oscillator's
    /// arithmetic identical to the product it used to be.
    anchor_t: f64,
    /// Musical position at `anchor_t`, phase shifts included.
    anchor_beats: f64,
    /// Musical position at `anchor_t`, excluding phase shifts (read by noise generators).
    anchor_elapsed: f64,
    /// Steps advanced so far. `t` is the f64 sum those steps produced and this
    /// is the count they came in as; the two are not interchangeable, which is
    /// exactly why both are kept — see [`Oscillator::steps_taken`].
    steps_taken: u64,
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
        Oscillator {
            bpm: clamp_bpm(bpm),
            t: 0.0,
            anchor_t: 0.0,
            anchor_beats: 0.0,
            anchor_elapsed: 0.0,
            steps_taken: 0,
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
        self.steps_taken += u64::from(steps);
    }

    /// Returns a copy of the oscillator with elapsed time shifted back by `seconds`.
    ///
    /// Extrapolates the current tempo grid backwards without altering anchor points.
    /// Used for warming off-air slots running behind the live session.
    pub fn behind(self, seconds: f64) -> Oscillator {
        Oscillator {
            t: self.t - seconds,
            ..self
        }
    }

    /// Returns a copy of the oscillator evaluated at the specified absolute simulation time `t`.
    pub fn at_time(self, t: f64) -> Oscillator {
        Oscillator { t, ..self }
    }

    /// Returns the total simulation steps this oscillator has advanced.
    pub fn steps_taken(&self) -> u64 {
        self.steps_taken
    }

    /// Applies a tempo and phase shift correction.
    ///
    /// Updates the tempo and shifts the beat grid continuously without moving
    /// historical beats. The phase shift applies to [`Oscillator::beats`] but
    /// does not alter [`Oscillator::elapsed_beats`].
    pub fn correct(&mut self, bpm: f32, shift_beats: f32) {
        let shift = if shift_beats.is_finite() {
            shift_beats as f64
        } else {
            0.0
        };
        self.anchor_beats = self.beats() + shift;
        self.anchor_elapsed = self.elapsed_beats();
        self.anchor_t = self.t;
        self.bpm = clamp_bpm(bpm);
    }

    /// The oscillator's tempo — free-running from `--bpm`, or whatever
    /// [`Oscillator::correct`] last set. The local oscillator is still the
    /// single source of truth: an estimate corrects this, nothing reads past
    /// it.
    pub fn bpm(&self) -> f32 {
        self.bpm
    }

    /// Musical position in beats, phase shifts included. Monotone while the
    /// tempo is positive, and continuous across a tempo correction.
    pub fn beats(&self) -> f64 {
        self.anchor_beats + self.since_anchor()
    }

    /// Musical position in beats with phase shifts excluded — the one noise
    /// reads. See the module doc.
    pub fn elapsed_beats(&self) -> f64 {
        self.anchor_elapsed + self.since_anchor()
    }

    fn since_anchor(&self) -> f64 {
        (self.t - self.anchor_t) * self.bpm as f64 / 60.0
    }

    /// Elapsed simulation time in seconds, `sum(steps * dt)` across every call
    /// to [`advance`](Oscillator::advance) so far. Never wall clock.
    pub fn t(&self) -> f64 {
        self.t
    }

    /// Position within the current beat, `0.0..1.0`. `0.0` is the instant of
    /// the beat; the value rises linearly and wraps at the next beat.
    pub fn beat_phase(&self) -> f32 {
        self.beats().rem_euclid(1.0) as f32
    }

    /// Position within the current bar, `0.0..1.0`, using [`BEATS_PER_BAR`].
    pub fn bar_phase(&self) -> f32 {
        (self.beats() / BEATS_PER_BAR as f64).rem_euclid(1.0) as f32
    }
}

/// A NaN tempo becomes the low bound rather than poisoning every phase
/// downstream, and every tempo is held inside [`BPM_RANGE`] — including one
/// arriving from a correction, where a wild estimate is a thing that happens.
fn clamp_bpm(bpm: f32) -> f32 {
    if bpm.is_nan() {
        *BPM_RANGE.start()
    } else {
        bpm.clamp(*BPM_RANGE.start(), *BPM_RANGE.end())
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
mod correction_tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn run(seconds: f32) -> Oscillator {
        let mut osc = Oscillator::new(120.0);
        for _ in 0..(seconds / DT) as u32 {
            osc.advance(1, DT);
        }
        osc
    }

    /// The reason musical position is an accumulator: a tempo correction
    /// changes the rate from now on and leaves the beat that is happening where
    /// it is. Multiplying `t` by a new tempo would slide every beat that had
    /// already happened, which on stage is the picture jumping when the tracker
    /// merely sharpens its estimate.
    #[test]
    fn a_tempo_correction_does_not_move_the_phase_it_arrives_at() {
        let mut osc = run(10.0);
        let before = osc.beat_phase();
        osc.correct(128.0, 0.0);
        assert!(
            (osc.beat_phase() - before).abs() < 1e-6,
            "phase jumped from {before} to {} on a tempo change",
            osc.beat_phase()
        );
        assert_eq!(osc.bpm(), 128.0);

        // ...and from there it runs at the new rate.
        osc.advance(1, 60.0 / 128.0);
        assert!(
            (osc.beat_phase() - before).abs() < 1e-5,
            "one beat at the corrected tempo did not return to the same phase"
        );
    }

    /// A phase shift moves the grid by exactly what it says, and forward means
    /// sooner — the direction a late oscillator has to move.
    #[test]
    fn a_phase_shift_moves_the_grid_by_what_it_says_and_forward_means_sooner() {
        let mut osc = run(10.0);
        let before = osc.beats();
        osc.correct(osc.bpm(), 0.25);
        assert!((osc.beats() - before - 0.25).abs() < 1e-9);

        // "Sooner": the next beat instant is a quarter of a beat closer.
        // Modulo one beat, because "closer" past a beat boundary means the beat
        // after it — a shift of a quarter beat from a phase of 0.9 lands on the
        // next beat rather than a negative distance to this one.
        let to_beat_before = 1.0 - before.rem_euclid(1.0);
        let to_beat_after = 1.0 - osc.beats().rem_euclid(1.0);
        assert!(
            ((to_beat_before - to_beat_after).rem_euclid(1.0) - 0.25).abs() < 1e-6,
            "a forward shift did not bring the next beat closer: {to_beat_before} -> {to_beat_after}"
        );
    }

    /// An uncorrected oscillator is the one that existed before corrections
    /// did, bit for bit — the accumulator is anchored at zero, so it reduces to
    /// the product it replaced.
    #[test]
    fn an_uncorrected_oscillator_is_the_product_it_used_to_be_bit_for_bit() {
        for bpm in [90.0_f32, 120.0, 128.5] {
            let mut osc = Oscillator::new(bpm);
            for i in 0..600 {
                osc.advance(1 + (i % 3) as u8, DT);
                let product = osc.t() * bpm as f64 / 60.0;
                assert_eq!(osc.beats(), product, "beats drifted from the product");
                assert_eq!(osc.elapsed_beats(), product);
                assert_eq!(osc.beat_phase(), product.rem_euclid(1.0) as f32);
            }
        }
    }

    /// Answered in the type: noise follows a
    /// tempo correction and ignores a phase one.
    #[test]
    fn noise_time_follows_a_tempo_correction_and_ignores_a_phase_one() {
        let mut osc = run(10.0);
        let elapsed = osc.elapsed_beats();

        osc.correct(osc.bpm(), 0.4);
        assert_eq!(
            osc.elapsed_beats(),
            elapsed,
            "a phase correction reached the noise clock"
        );
        assert_ne!(osc.beats(), elapsed, "a phase correction did nothing");

        // A tempo correction is a rate change, and noise time takes it —
        // continuously, with no jump at the instant it lands.
        osc.correct(240.0, 0.0);
        assert_eq!(
            osc.elapsed_beats(),
            elapsed,
            "noise time jumped on a rate change"
        );
        osc.advance(1, 1.0);
        assert!(
            (osc.elapsed_beats() - elapsed - 4.0).abs() < 1e-9,
            "noise time did not run at the corrected tempo"
        );
    }

    /// A tracker that goes mad cannot stop or reverse the session clock.
    #[test]
    fn a_wild_correction_is_clamped_like_any_other_tempo() {
        let mut osc = run(1.0);
        for bad in [0.0, -400.0, f32::NAN, f32::INFINITY, 1e9] {
            osc.correct(bad, 0.0);
            assert!(BPM_RANGE.contains(&osc.bpm()), "{bad} survived clamping");
        }
        // And a nonsense shift is dropped rather than making every phase NaN.
        let beats = osc.beats();
        osc.correct(osc.bpm(), f32::NAN);
        assert_eq!(osc.beats(), beats);
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
