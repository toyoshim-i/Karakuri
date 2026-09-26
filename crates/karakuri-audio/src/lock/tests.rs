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
                    half_tempo_hint: false,
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

/// Verifies that the phase-locked loop aligns visible beats with music beats,
/// properly compensating for both analysis and output latency.
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

/// A single conflicting estimate must not perturb grid phase or tempo.
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

// -- the octave ---------------------------------------------------------

/// Verifies that manual octave shifts immediately scale BPM, preserve phase,
/// and discard obsolete estimates from the previous octave window.
#[test]
fn the_octave_control_moves_the_grid_and_survives_the_old_octave_s_estimates() {
    let bpm = 87.0;
    let mut session = Session::new(120.0);
    session.run(10.0, Some((bpm, 0.9)), true);
    assert!(session.lock.locked());
    let phase = session.oscillator.beat_phase();

    let correction = session
        .lock
        .octave(2.0, &session.oscillator)
        .expect("174 bpm is inside the range");
    assert_eq!(correction.bpm, 174.0);
    assert_eq!(correction.shift, 0.0);
    assert_eq!(session.lock.reason(), Some(Reason::Octave));
    session.oscillator.correct(correction.bpm, correction.shift);
    assert_eq!(
        session.oscillator.beat_phase(),
        phase,
        "an octave move shifted the phase"
    );

    // A second of the old octave's estimates still arriving — the tracker
    // re-measures four times a second, so this is four times the exposure
    // the real one has — and the grid holds the octave it was given.
    session.run(1.0, Some((bpm, 0.9)), true);
    assert!(
        (session.oscillator.bpm() - 174.0).abs() < 1.0,
        "the old octave pulled the grid back to {} within a second",
        session.oscillator.bpm()
    );
}

/// Verifies that an octave shift clears pending relock disagreement counters.
#[test]
fn an_octave_move_discards_the_run_of_disagreement_it_interrupts() {
    let mut session = Session::new(120.0);
    session.run(10.0, Some((87.0, 0.9)), true);
    assert!(session.lock.locked());

    // Seven consistent disagreeing revisions: one short of RELOCK_EVIDENCE.
    session.run(
        0.25 * (RELOCK_EVIDENCE - 1) as f32,
        Some((100.0, 0.9)),
        true,
    );
    assert!(
        (session.oscillator.bpm() - 87.0).abs() < 1.0,
        "the run re-acquired before the key was pressed, so this proves nothing"
    );

    let correction = session
        .lock
        .octave(2.0, &session.oscillator)
        .expect("174 bpm is inside the range");
    session.oscillator.correct(correction.bpm, correction.shift);

    // The estimates that would have completed the run keep arriving.
    session.run(0.75, Some((100.0, 0.9)), true);
    assert!(
        (session.oscillator.bpm() - 174.0).abs() < 1.0,
        "a run that started against the old grid re-acquired the new one at {}",
        session.oscillator.bpm()
    );
}

/// Verifies that the advisory half-tempo hint does not affect oscillator tracking.
#[test]
fn the_half_tempo_hint_does_not_reach_the_grid() {
    let run = |hint: bool| {
        let mut session = Session::new(120.0);
        for _ in 0..(12.0 / DT) as u32 {
            session.frame(Some((128.0, 0.9)), true);
            session.estimate.half_tempo_hint = hint;
        }
        (
            session.oscillator.bpm(),
            session.oscillator.beats(),
            session.lock.locked(),
        )
    };
    assert_eq!(run(true), run(false));
}

/// A move that would leave the trackable range is refused rather than
/// applied and then undone by the next estimate that disagrees.
#[test]
fn an_octave_move_out_of_range_is_refused() {
    let oscillator = Oscillator::new(174.0);
    let mut lock = BeatLock::new();
    assert!(lock.octave(2.0, &oscillator).is_none(), "348 bpm was taken");
    assert_eq!(lock.reason(), None);
    assert!(
        lock.octave(0.5, &oscillator).is_some(),
        "87 bpm was refused"
    );

    let slow = Oscillator::new(90.0);
    assert!(lock.octave(0.5, &slow).is_none(), "45 bpm was taken");
}

// -- a tempo named by hand ----------------------------------------------

/// Verifies that manually setting a target tempo clears evidence counters
/// without mutating current lock state.
#[test]
fn a_tempo_named_by_hand_clears_the_run_and_says_nothing_about_the_lock() {
    // A tracked grid three revisions short of being dragged back.
    let mut tracked = BeatLock::new();
    tracked.state = State::Locked;
    tracked.disagreement = RELOCK_EVIDENCE - 3;
    tracked.candidate_bpm = 160.0;
    tracked.error = 0.2;
    tracked.retarget(145.0);
    assert_eq!(
        tracked.disagreement, 0,
        "the run behind the old target survived a set"
    );
    assert_eq!(tracked.agreement, 0);
    assert_eq!(
        tracked.candidate_bpm, 145.0,
        "the next estimate would be counted against a tempo nobody asked for"
    );
    assert_eq!(
        tracked.error, 0.0,
        "a phase error measured against a tempo that is gone is still on the panel"
    );
    assert!(
        tracked.locked(),
        "a set unlocked a grid that is being tracked"
    );

    // A free-running grid one revision short of acquiring.
    let mut free = BeatLock::new();
    free.agreement = ACQUIRE_EVIDENCE - 1;
    free.candidate_bpm = 128.0;
    free.retarget(145.0);
    assert_eq!(
        free.agreement, 0,
        "the run behind the old target survived a set"
    );
    assert!(
        !free.locked(),
        "a set said the grid was locked to something"
    );

    // A number that is not a tempo leaves everything alone rather than
    // poisoning the comparison every run is counted by.
    let mut poisoned = BeatLock::new();
    poisoned.candidate_bpm = 128.0;
    for bad in [f32::NAN, f32::INFINITY, 0.0, -128.0] {
        poisoned.retarget(bad);
        assert_eq!(poisoned.candidate_bpm, 128.0, "{bad} was taken as a tempo");
    }
}

/// Verifies that manual retargeting requires a full run of fresh evidence
/// before automatic relocking can override it.
#[test]
fn a_tempo_named_by_hand_costs_the_room_a_whole_run_to_take_the_grid_back() {
    // Estimates until the grid moves again, from a session locked to one
    // tempo and then argued with by another.
    let revisions_until_the_room_wins = |clear: bool| {
        let mut session = Session::new(120.0);
        session.run(12.0, Some((128.0, 0.9)), true);
        assert!(session.lock.locked(), "the grid never locked to the room");

        // The room is at another tempo now, and has said so five times.
        while session.lock.disagreement < 5 {
            session.frame(Some((160.0, 0.9)), true);
        }

        // The press: the operator names a tempo, and the grid moves
        // because of the record rather than because of this call.
        if clear {
            session.lock.retarget(145.0);
        }
        session.oscillator.correct(145.0, 0.0);

        let mut revisions = 0;
        let mut seen = session.lock.seen;
        while (session.oscillator.bpm() - 145.0).abs() < 0.001 {
            session.frame(Some((160.0, 0.9)), true);
            if session.lock.seen != seen {
                seen = session.lock.seen;
                revisions += 1;
            }
            assert!(revisions < 40, "the room never took the grid back");
        }
        revisions
    };

    assert_eq!(
        revisions_until_the_room_wins(true),
        RELOCK_EVIDENCE,
        "a hand-set tempo was dragged back in less than a full run of disagreement"
    );
    assert_eq!(
        revisions_until_the_room_wins(false),
        RELOCK_EVIDENCE - 5,
        "the run left over from before the press is no longer what carries the grid back, \
             so this test is no longer measuring the clearing"
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
