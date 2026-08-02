//! The whole chain, on synthesised audio: samples → analyser → tracker → lock →
//! the local oscillator.
//!
//! Every other test in this crate covers one link. This one covers the joins,
//! which is where the lags live — the unit tests for [`karakuri_audio::lock`]
//! hand it estimates that are correct by construction, and could not catch a
//! device path that mislabels *when* an estimate refers to.
//!
//! Nothing here opens a device. Audio arrives on a simulated clock, an
//! artificial input latency in front of it, and the assertion is what an
//! audience would see: the oscillator's phase at the instant this frame becomes
//! light, against the music at that same instant.

use karakuri_audio::analysis::{Analyzer, BLOCK, HOP};
use karakuri_audio::lock::{wrap_beats, BeatLock};
use karakuri_audio::tempo::{Estimate, Tracker};
use karakuri_signal::Oscillator;

const RATE: usize = 48_000;
const DT: f32 = 1.0 / 60.0;

/// A device buffer of 256 frames at 48 kHz, near enough — the number is not
/// what is being tested, the fact that it is compensated is.
const INPUT_LATENCY: f32 = 256.0 / RATE as f32;

/// The output lag: two frames of queue plus a display's own pipeline. In the
/// CLI this is an operator dial; here it is a number the test knows and the
/// code under test only sees as a duration.
const OUTPUT_LAG: f32 = 2.0 * DT + 0.012;

/// A click train at `bpm` with a click at sample zero, so the beats are at
/// exactly `k * 60 / bpm` seconds.
fn clicks(bpm: f32, seconds: f32) -> Vec<f32> {
    let period = (60.0 / bpm * RATE as f32) as usize;
    let len = (seconds * RATE as f32) as usize;
    let mut samples = vec![0.0; len];
    let mut at = 0;
    while at < len {
        for n in 0..96.min(len - at) {
            // A short burst with some spectral width, which is what an onset
            // detector is looking for.
            let decay = 1.0 - n as f32 / 96.0;
            samples[at + n] = 0.8 * decay * if n % 2 == 0 { 1.0 } else { -1.0 };
        }
        at += period;
    }
    samples
}

fn music_phase(bpm: f32, at: f64) -> f32 {
    (at * bpm as f64 / 60.0).rem_euclid(1.0) as f32
}

struct Rig {
    analyzer: Analyzer,
    tracker: Tracker,
    lock: BeatLock,
    oscillator: Oscillator,
    estimate: Estimate,
    published: f64,
    next_block: usize,
    now: f64,
}

impl Rig {
    fn new(free_running_bpm: f32) -> Rig {
        let analyzer = Analyzer::new(RATE as u32);
        // The window starts centred on the session tempo, which is the same
        // number the oscillator free-runs at — one number doing both jobs, as
        // it does on the command line.
        let tracker = Tracker::new(analyzer.hop_seconds(), analyzer.window_lag(), free_running_bpm);
        Rig {
            analyzer,
            tracker,
            lock: BeatLock::new(),
            oscillator: Oscillator::new(free_running_bpm),
            estimate: Estimate::unknown(0.0),
            published: f64::NEG_INFINITY,
            next_block: 0,
            now: 0.0,
        }
    }

    /// One rendered frame, with whatever audio has arrived by now.
    fn frame(&mut self, samples: &[f32], lead: bool) {
        // The device delivers a sample `INPUT_LATENCY` after it happened.
        let available = (((self.now - f64::from(INPUT_LATENCY)) * RATE as f64) as isize).max(0)
            as usize;
        while self.next_block + BLOCK <= available.min(samples.len()) {
            let analysis = self
                .analyzer
                .analyze(&samples[self.next_block..self.next_block + BLOCK]);
            // What `device` does on every hop: the tracking window is centred
            // on the grid, so the octave follows the tempo the grid is running.
            self.tracker.set_centre_bpm(self.oscillator.bpm());
            self.tracker.push(analysis.novelty);
            self.estimate = self.tracker.estimate();
            self.published = self.now;
            self.next_block += HOP;
        }

        if self.published.is_finite() {
            let age = (self.now - self.published) as f32
                + INPUT_LATENCY
                + self.analyzer.window_lag();
            let ahead = if lead { age + OUTPUT_LAG } else { 0.0 };
            if let Some(c) = self
                .lock
                .update(&self.estimate, ahead, &self.oscillator, DT)
            {
                self.oscillator.correct(c.bpm, c.shift);
            }
        }

        self.oscillator.advance(1, DT);
        self.now += f64::from(DT);
    }

    fn run(&mut self, samples: &[f32], seconds: f32, lead: bool) {
        for _ in 0..(seconds / DT) as u32 {
            self.frame(samples, lead);
        }
    }

    /// What the audience sees.
    fn visible_error(&self, bpm: f32) -> f32 {
        wrap_beats(self.oscillator.beat_phase() - music_phase(bpm, self.now + OUTPUT_LAG as f64))
    }
}

/// The whole point, end to end: real analysis, a real estimate, a real device
/// delay in front of it — and the beat lands on the beat.
///
/// The second half of the test is what keeps the first half honest. The same
/// rig with the lead removed has to be late by `A + D`, or the assertion above
/// would also pass against a loop that compensated for nothing.
#[test]
fn a_click_train_puts_the_oscillators_beat_on_the_music_s_beat() {
    let bpm = 128.0;
    let samples = clicks(bpm, 24.0);

    let mut led = Rig::new(120.0);
    led.run(&samples, 20.0, true);
    assert!(led.lock.locked(), "the grid never locked to a click train");
    assert!(
        (led.oscillator.bpm() - bpm).abs() < 1.0,
        "the grid settled at {} bpm",
        led.oscillator.bpm()
    );
    let error = led.visible_error(bpm);
    assert!(
        error.abs() < 0.02,
        "the visible beat is {error} beats out ({} ms)",
        error * 60.0 / bpm * 1000.0
    );

    let mut naive = Rig::new(120.0);
    naive.run(&samples, 20.0, false);
    let late = naive.visible_error(bpm);
    let lag = INPUT_LATENCY + naive.analyzer.window_lag() + OUTPUT_LAG;
    let expected = lag * bpm / 60.0;
    assert!(
        late.abs() > expected * 0.5,
        "without the lead the picture should be about {expected} beats late; it was {late}"
    );
}

/// **The octave, end to end, in both of its halves.**
///
/// A 174 bpm track under a session started at 87 locks at 87 — *confidently*,
/// because the grid does fit the music, every other pulse of it. Nothing
/// automatic will move it and nothing here pretends otherwise: the tracker
/// centres its one-octave window on the grid, so the grid is what decides which
/// octave gets tracked, and it never disagrees with itself.
///
/// Then the operator presses ×2, and the second half of the test is the part
/// that matters: the grid moves, the window moves with it, and ten seconds
/// later it is *still* at 174 with the beat on the beat — rather than folding
/// straight back the moment the next estimate arrives, which is what would
/// happen if the window had not come along.
#[test]
fn the_octave_key_moves_the_grid_and_the_window_and_tracking_continues_there() {
    let bpm = 174.0;
    let samples = clicks(bpm, 40.0);

    let mut rig = Rig::new(bpm / 2.0);
    rig.run(&samples, 16.0, true);
    assert!(rig.lock.locked(), "the grid never locked at all");
    assert!(
        (rig.oscillator.bpm() - bpm / 2.0).abs() < 2.0,
        "a window centred an octave low settled at {} bpm, not at {}",
        rig.oscillator.bpm(),
        bpm / 2.0
    );

    let correction = rig
        .lock
        .octave(2.0, &rig.oscillator)
        .expect("174 bpm is inside the tracked range");
    rig.oscillator.correct(correction.bpm, correction.shift);
    assert_eq!(rig.oscillator.bpm(), correction.bpm);
    assert!((rig.oscillator.bpm() - bpm).abs() < 2.0);

    rig.run(&samples, 10.0, true);
    assert!(
        (rig.oscillator.bpm() - bpm).abs() < 2.0,
        "the grid folded back to {} bpm after the octave key",
        rig.oscillator.bpm()
    );
    let error = rig.visible_error(bpm);
    assert!(
        error.abs() < 0.02,
        "in the new octave the visible beat is {error} beats out"
    );
    assert!(rig.lock.locked());
}

/// The measured signals come out of the same path, and a click train is not
/// silence: `energy` moves, `onset` fires, and the frame is fully believed
/// because a measurement happened.
#[test]
fn the_same_path_produces_measured_signals_at_full_confidence() {
    let samples = clicks(128.0, 4.0);
    let mut analyzer = Analyzer::new(RATE as u32);
    let mut at = 0;
    let mut peak_energy = 0.0f32;
    let mut peak_onset = 0.0f32;
    while at + BLOCK <= samples.len() {
        let analysis = analyzer.analyze(&samples[at..at + BLOCK]);
        assert_eq!(analysis.frame.confidence, 1.0);
        peak_energy = peak_energy.max(analysis.frame.energy);
        peak_onset = peak_onset.max(analysis.frame.onset);
        at += HOP;
    }
    assert!(peak_energy > 0.3, "a click train read {peak_energy} energy");
    assert_eq!(peak_onset, 1.0, "no onset fired on a click train");
}

/// A smoke test against **real hardware**, ignored by default.
///
/// `cargo test -p karakuri-audio -- --ignored --nocapture` opens the default
/// input, reads it for a second, and prints what came back. It is ignored
/// because a test that needs a microphone is a test that gets skipped —
/// everything above this line is the actual coverage, and it needs no device.
/// What this catches is the half no synthesised test can: whether a device
/// opens at all, at what rate, and whether frames arrive.
#[test]
#[ignore = "needs an audio input device"]
fn the_default_input_opens_and_delivers() {
    let mut input = match karakuri_audio::AudioInput::open("default", 120.0) {
        Ok(input) => input,
        Err(e) => panic!("could not open the default input: {e}"),
    };
    eprintln!("opened {} at {} Hz", input.description(), input.sample_rate());
    let mut best = 0.0f32;
    for _ in 0..100 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        let reading = input.read();
        best = best.max(reading.frame.confidence);
        eprintln!(
            "  energy {:.3} onset {:.3} confidence {:.2} age {:.3}s bpm {:.1} c{:.2}",
            reading.frame.energy,
            reading.frame.onset,
            reading.frame.confidence,
            reading.age,
            reading.estimate.bpm,
            reading.estimate.confidence,
        );
    }
    assert!(
        best > 0.0,
        "the device opened but delivered nothing in a second"
    );
}


/// **An octave-low grid has to be steady as well as wrong.** Half of a click
/// train's pulses are on that grid and half are between them, and the two sets
/// are equally good beats — nothing in the novelty says which is the downbeat.
/// Picking the other one on some later window would yank the picture half a
/// beat, which is worse than the octave itself; the phase comes off the
/// strongest hump of a folded profile, and the same hump has to keep winning.
///
/// Half a minute of it, measured as jumps rather than as a final position: a
/// grid that ends where it started could still have gone round the houses.
#[test]
fn a_grid_left_an_octave_low_does_not_change_its_mind_about_which_pulse_is_the_beat() {
    let bpm = 174.0;
    let samples = clicks(bpm, 60.0);
    let mut rig = Rig::new(bpm / 2.0);
    rig.run(&samples, 16.0, true);
    assert!(rig.lock.locked());

    let mut jumped = Vec::new();
    let mut last = rig.oscillator.beats();
    for _ in 0..(30.0 / DT) as u32 {
        rig.frame(&samples, true);
        let moved = rig.oscillator.beats() - last;
        // What one frame of a free-running grid at this tempo is worth. The
        // trim is a thousandth of a beat a frame; anything near a hundredth is
        // a re-acquire, and anything near a half is the parity flipping.
        let expected = rig.oscillator.bpm() as f64 / 60.0 * DT as f64;
        if (moved - expected).abs() > 0.01 {
            jumped.push((rig.now, moved - expected));
        }
        last = rig.oscillator.beats();
    }
    assert!(
        jumped.is_empty(),
        "the octave-low grid jumped (at, beats): {jumped:?}"
    );
    assert!((rig.oscillator.bpm() - bpm / 2.0).abs() < 2.0);
}
