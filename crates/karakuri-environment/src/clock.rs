//! Derives simulation step counts from real-time intervals (P-0092, ADR-0006).

use std::time::Instant;

use karakuri_engine::set::DT;
use karakuri_store::record::MAX_STEPS;

/// Tracks elapsed wall-clock time and converts intervals into simulation step counts.
pub struct Clock {
    last: Instant,
    /// Fractional steps carried between frames to preserve average pacing across frame rates.
    carry: f32,
    /// The last measured frame interval.
    interval: f32,
}

impl Clock {
    pub fn new(now: Instant) -> Clock {
        Clock {
            last: now,
            carry: 0.0,
            interval: DT,
        }
    }

    /// Advances the clock to `now` and returns the number of simulation steps to run,
    /// clamped to `MAX_STEPS` (ADR-0006, ADR-0078).
    pub fn steps(&mut self, now: Instant) -> u8 {
        let elapsed = now.duration_since(self.last).as_secs_f32();
        self.last = now;
        self.interval = elapsed;

        self.carry += elapsed / DT;
        let whole = self.carry.floor();
        self.carry -= whole;
        (whole as u32).min(u32::from(MAX_STEPS)) as u8
    }

    /// Returns the last measured frame interval in seconds.
    pub fn interval(&self) -> f32 {
        self.interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Measures simulation rate across refresh rates to verify it maintains 1.0 sim sec / wall sec (ADR-0297).
    const SECONDS: f64 = 10.0;

    fn simulated_seconds_a_second(hz: f64) -> f64 {
        let start = Instant::now();
        let mut clock = Clock::new(start);
        let frames = (hz * SECONDS).round() as u64;
        let mut steps = 0u32;
        for frame in 1..=frames {
            let at = start + Duration::from_secs_f64(frame as f64 / hz);
            steps += u32::from(clock.steps(at));
        }
        f64::from(steps) * f64::from(DT) / SECONDS
    }

    #[test]
    fn the_room_advances_at_one_second_a_second_whatever_the_display_does() {
        for hz in [30.0, 40.53, 60.0, 75.0, 120.0, 144.0] {
            let rate = simulated_seconds_a_second(hz);
            assert!(
                (rate - 1.0).abs() < 0.002,
                "a {hz} Hz display advanced the simulation {rate:.3} seconds a second — \
                 the step count is derived from the interval precisely so that the \
                 refresh rate is not the tempo"
            );
        }
    }

    /// A frame that does not divide the step rate spends its remainder later, which
    /// is the carry and is why the assertion above can be exact rather than
    /// approximate. At 120 Hz half the frames advance nothing at all, and that is
    /// the right answer rather than a rounding loss.
    #[test]
    fn a_frame_shorter_than_a_step_advances_nothing_and_the_next_one_pays() {
        let start = Instant::now();
        let mut clock = Clock::new(start);
        let half = Duration::from_secs_f32(DT / 2.0);
        assert_eq!(clock.steps(start + half), 0);
        assert_eq!(clock.steps(start + half * 2), 1);
    }

    /// The interval is the one the step count was taken from, not a second reading
    /// of the clock.
    #[test]
    fn the_interval_is_the_measurement_the_steps_came_from() {
        let start = Instant::now();
        let mut clock = Clock::new(start);
        clock.steps(start + Duration::from_millis(25));
        assert!((clock.interval() - 0.025).abs() < 1e-6);
    }

    // Tests migrated under ADR-0297.

    /// Verifies that clock intervals accumulate across paused or skipped frames
    /// so the total elapsed time matches regardless of frame cadence.
    #[test]
    fn a_frame_that_never_ran_leaves_its_time_for_the_next_one() {
        let start = Instant::now();
        let ms = |n: u64| start + std::time::Duration::from_millis(n);

        let mut drew_every_frame = Clock::new(start);
        let both =
            u32::from(drew_every_frame.steps(ms(16))) + u32::from(drew_every_frame.steps(ms(32)));

        let mut skipped_one = Clock::new(start);
        let one = u32::from(skipped_one.steps(ms(32)));

        assert_eq!(
            one, both,
            "the abandoned frame's interval was dropped rather than carried"
        );
        assert!(both > 0, "thirty-two milliseconds is at least one step");
    }

    /// The carry is what makes that true across a frame rate that does not divide
    /// the step rate: whole steps out, the fraction kept.
    #[test]
    fn the_clock_hands_out_whole_steps_and_keeps_the_fraction() {
        let start = Instant::now();
        let mut clock = Clock::new(start);
        let mut total = 0u32;
        for i in 1..=60u64 {
            total += u32::from(clock.steps(start + std::time::Duration::from_millis(i * 16)));
        }
        let expected = (0.960 / f64::from(DT)).floor() as u32;
        assert_eq!(
            total, expected,
            "the fraction between frames was dropped: {total} steps for 960 ms"
        );
    }

    /// Verifies that elapsed intervals exceeding MAX_STEPS clamp to MAX_STEPS rather than overflowing.
    #[test]
    fn a_long_gap_falls_behind_rather_than_catching_up() {
        let start = Instant::now();
        let mut clock = Clock::new(start);
        let steps = clock.steps(start + std::time::Duration::from_secs(5));
        assert_eq!(steps, MAX_STEPS, "five seconds is not four steps' worth");
    }
}
