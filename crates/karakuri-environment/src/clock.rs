//! Real time, as the one number a frame is allowed to derive from it: how many
//! simulation steps it advances by.
//!
//! # Why this is here and not in a surface
//!
//!
//! [P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)'s
//! first sentence is the whole of this module's charter: *"Time comes from a
//! record: live, the engine derives the step count from real time and writes it
//! in; replaying, it reads the number back and derives nothing."* The
//! derivation is the live half of that rule, and it is one derivation — a
//! second copy of it is a second answer to a determinism rule, which is the
//! shape this repository spends records on removing.
//!
//!
//! `docs/adr/0215-the-package-is-karakuri-environment-and-a-module-belongs-if-what-it-deals-with-is-outside-this-process.md`
//! had already decided where it lives and this module is that instruction
//! carried out: applying the charter to `Clock::steps(&mut self, now: Instant)`
//! it wrote *"wall-clock time comes from outside this process"*, concluded
//! `Clock` moves and `Live` does not, and `lib.rs` has said since that the move
//! was owed and had not happened. It had not happened because one program
//! needed it; a second one needs it now
//! (`docs/adr/0297-the-panels-tick-is-measured-and-the-fixed-step-a-frame-ran-the-room-at-the-displays-rate.md`).
//!
//! # The clock read is the exception, and this is where it is spent
//!
//! P-0092 permits a clock to be read *"to judge cost"* and forbids a value
//! derived from one reaching simulation state. A step count reaches simulation
//! state, so this is not that exception — it is the rule's own live path: the
//! number is derived here, written into a `tick`, and the engine is advanced by
//! the record. What makes a replay frame-exact is that the same field is read
//! back rather than derived again.

use std::time::Instant;

use karakuri_engine::set::DT;
use karakuri_store::record::MAX_STEPS;

/// The one measurement a frame owes the record stream: elapsed real time as a
/// step count, which is exactly what a `tick` record carries.
///
/// Its own type so that reading it borrows only itself. A frame acquires a
/// target, and only then commits — measuring the clock among other things — and
/// in both programs the committing work is a closure holding the deck and the
/// recorder. A method on the surface's own assembly would have borrowed all of
/// it at once and the closure could not be written. Splitting the clock out is
/// what makes the ordering expressible, and it is also what let it move here.
///
/// `steps` is told what time it is rather than asking. One line of plumbing,
/// and it is what makes the thing this type exists to guarantee checkable: that
/// a frame which does not draw leaves its interval for the next one instead of
/// consuming it. With `Instant::now()` inside, a test could state no elapsed
/// time and could therefore assert nothing but tautologies — which is exactly
/// what the first test written against it did. It is also the property ADR-0215
/// read the placement off: *"the thing being passed in from outside is
/// precisely what makes it belong"*.
pub struct Clock {
    last: Instant,
    /// Fractional steps carried between frames, so a frame rate that does not
    /// divide the step rate still advances at the right average rate. The same
    /// accumulator shape as spawn quantisation, for the same reason.
    ///
    /// It is what makes a display's refresh rate stop deciding the tempo. A 120 Hz
    /// frame is half a step, so the carry spends it on every second frame and the
    /// room advances at `DT` a step either way.
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

    /// How many steps to advance by, called once by every frame that commits and
    /// never by one that does not.
    ///
    /// Where it is called from is ADR-0078: a frame that is discarded must not
    /// already have been recorded, so this is read after the last point at which a
    /// frame can be abandoned. `last` moves here and nowhere else, and that is what
    /// makes a gap survive: the frame loop not running at all — a paused event
    /// loop, a window the system stopped asking to redraw, a console folded down to
    /// nothing that draws — is counted whole by the next frame, up to
    /// [`MAX_STEPS`].
    ///
    /// The cap is the stream's, `karakuri_store::record::MAX_STEPS`, and not a
    /// constant re-typed beside the caller — the number is being written into a
    /// `tick` and the record's own vocabulary is what says how large one may be.
    ///
    ///
    /// [ADR-0006](../../../docs/adr/0006-the-step-count-is-a-record-not-a-measurement.md)
    /// decided both the cap and what it means when it engages: past it *"the
    /// simulation is allowed to fall behind, because unbounded catch-up turns a
    /// load spike into a death spiral"*, and `t` diverges from wall clock
    /// permanently and never resynchronizes — which that record states in the
    /// specification *"because otherwise it is reported as a bug"*
    /// (`docs/ir-spec.md`, *On `dt` and simulation time*). A caller does not get to
    /// soften either half: what is dropped here is dropped, and the `tick` says so.
    pub fn steps(&mut self, now: Instant) -> u8 {
        let elapsed = now.duration_since(self.last).as_secs_f32();
        self.last = now;
        // Kept because the output lag a beat correction leads by starts with
        // the frame queue, which is a number of *frames* — and the frame rate
        // is the display's, not `dt`'s. See `audio::Audio::output_lag`.
        self.interval = elapsed;

        self.carry += elapsed / DT;
        let whole = self.carry.floor();
        self.carry -= whole;
        (whole as u32).min(u32::from(MAX_STEPS)) as u8
    }

    /// The last frame interval, in seconds — the same measurement [`Clock::steps`]
    /// took, handed out rather than taken again.
    ///
    /// It is what the beat correction's output lag is built from, and the whole of
    /// why the interval is kept at all: the lag starts with the frame queue, which
    /// is a number of frames at the display's rate rather than at `DT`.
    pub fn interval(&self) -> f32 {
        self.interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// The simulation rate a display's refresh rate buys, which is the measurement
    /// ADR-0297 was written against: a frame rate is turned into simulation seconds
    /// per wall-clock second, and the answer has to be 1.0 whatever the display is
    /// doing.
    ///
    /// This is the derivation on its own, driven by stated instants — a display's
    /// refresh rate cannot be changed from a test, and this is the half of the
    /// chain that decides. The other half is `PresentMode::Fifo`: frames arrive at
    /// the display's rate, which is `Cost::wait`'s own sentence in
    /// `crates/karakuri/src/main.rs`. Ten seconds of frames, so that the one step
    /// still sitting in the carry when the window closes is under two parts in a
    /// thousand rather than under two in a hundred. The quantity is a rate, and a
    /// rate read over one period of the thing being counted is mostly that period.
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
        // A sag, the rate a console with every sink folded and the beat
        // declaring draws at (`BEAT_STALENESS`, 24.671 ms), the one the fixed
        // count was right at, and three displays that are on desks. Measured
        // on 2026-09-08: every one of them reads 0.9983 simulated seconds a
        // second, the shortfall being the one step still in the carry when
        // the window closes. A frame carrying a fixed 1 reads 0.5000, 0.6755,
        // 1.0000, 1.2500, 2.0000 and 2.4000 for the same six.
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

    // -- what came with the type -------------------------------------------
    //
    // These three were `karakuri-cli`'s and came here with [`Clock`] under
    // ADR-0297. They never touched a sink and never took a device — they are
    // about the derivation, which is what made the type movable and is now
    // two programs' rather than one program's. A test that stayed behind
    // would have been a test of one caller's copy.

    /// The interval of a frame that never ran is not lost — the next frame counts
    /// it.
    ///
    /// This used to be about a frame that found nowhere to draw, which was the only
    /// way the clock could go unread: `frame::compose` withheld the committing
    /// closure from a refused frame, so `Clock::steps` was not called and the
    /// interval carried. That is no longer a case at all — every frame
    /// `frame::compose` composes reads the clock, whatever the sinks answered — and
    /// the property it was checking is the same one, now carrying the gap where the
    /// frame loop itself does not run: a paused event loop, a window the operating
    /// system stopped sending redraws to, a long stall. The arithmetic below never
    /// mentioned a sink, which is why the assertion stands unchanged while its
    /// subject moved.
    ///
    /// The claim the frame loop's ordering rests on, and until `steps` could be
    /// told what time it is there was no way to state it: the first version of this
    /// test asserted that the step count did not exceed `MAX_STEPS` (it cannot:
    /// `steps` clamps to it) and that the carry was under one (it is: `steps`
    /// subtracts its own floor). Both survived deleting the body of `Clock::steps`.
    ///
    /// Two clocks over the same span, one reading it in two frames and one in a
    /// single frame because the other was abandoned, must hand out the same total.
    /// That is what "the time survives" means, and it is false for any clock that
    /// resets `last` somewhere other than a frame that goes ahead.
    #[test]
    fn a_frame_that_never_ran_leaves_its_time_for_the_next_one() {
        let start = Instant::now();
        let ms = |n: u64| start + std::time::Duration::from_millis(n);

        let mut drew_every_frame = Clock::new(start);
        let both =
            u32::from(drew_every_frame.steps(ms(16))) + u32::from(drew_every_frame.steps(ms(32)));

        // The same thirty-two milliseconds, with the frame at 16 ms never run
        // at all: `steps` is not called, so `last` does not move.
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
        // Sixty frames of 16 ms is 960 ms, and at `DT` per step that is a known
        // number of steps — known well enough that dropping the carry loses
        // several of them.
        for i in 1..=60u64 {
            total += u32::from(clock.steps(start + std::time::Duration::from_millis(i * 16)));
        }
        let expected = (0.960 / f64::from(DT)).floor() as u32;
        assert_eq!(
            total, expected,
            "the fraction between frames was dropped: {total} steps for 960 ms"
        );
    }

    /// And the anti-spiral clamp holds: a stall does not become a catch-up.
    #[test]
    fn a_long_gap_falls_behind_rather_than_catching_up() {
        // **The stream's cap, and there is no second copy of it to disagree
        // with.** `Clock::steps` clamps to `karakuri_store::record::MAX_STEPS`
        // itself now — a `tick` that named more steps than a reader will
        // accept is unreplayable, so the number the record format publishes is
        // the only one either side may hold. `karakuri-cli` kept its own `4`
        // beside this assertion until ADR-0297 and the two agreeing was what
        // this test checked; the clamp reads the published number directly, so
        // what is checked here is the clamp.
        let start = Instant::now();
        let mut clock = Clock::new(start);
        let steps = clock.steps(start + std::time::Duration::from_secs(5));
        assert_eq!(steps, MAX_STEPS, "five seconds is not four steps' worth");
    }
}
