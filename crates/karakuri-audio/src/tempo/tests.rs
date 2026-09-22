use super::*;
use crate::analysis::{Analyzer, BLOCK, HOP};
use std::f32::consts::TAU;

const RATE: u32 = 48_000;

/// A click train at `bpm`, `seconds` long, with a click at sample 0.
fn clicks(bpm: f32, seconds: f32, amplitude: f32) -> Vec<f32> {
    let period = (60.0 / bpm * RATE as f32) as usize;
    let len = (seconds * RATE as f32) as usize;
    let mut samples = vec![0.0; len];
    let mut at = 0;
    while at < len {
        for n in 0..64.min(len - at) {
            samples[at + n] = amplitude * (1.0 - n as f32 / 64.0);
        }
        at += period;
    }
    samples
}

/// Analyse a signal and feed the tracker, exactly as the device does, with
/// the tracking window centred at `centre` — which is the operator's
/// `--bpm` before there is a grid, so most tests pass the tempo they built
/// the signal at and the octave ones deliberately pass something else.
fn track(samples: &[f32], centre: f32) -> Tracker {
    let mut analyzer = Analyzer::new(RATE);
    let mut tracker = Tracker::new(analyzer.hop_seconds(), analyzer.window_lag(), centre);
    let mut block = vec![0.0f32; BLOCK];
    let mut at = 0;
    while at + BLOCK <= samples.len() {
        block.copy_from_slice(&samples[at..at + BLOCK]);
        tracker.push(analyzer.analyze(&block).novelty);
        at += HOP;
    }
    tracker
}

/// What an estimate costs, on a host clock, in the audio callback where it
/// runs. Ignored for the same reason as `analysis`'s: it is a measurement.
/// `cargo test -p karakuri-audio --release -- --ignored --nocapture`.
#[test]
#[ignore = "a measurement, not an assertion"]
fn what_one_estimate_costs() {
    let hop = HOP as f32 / RATE as f32;
    let mut tracker = Tracker::new(hop, BLOCK as f32 / 2.0 / RATE as f32, 120.0);
    for i in 0..2000 {
        tracker.push((i % 47) as f32 * 0.01);
    }
    // One push in every ESTIMATE_INTERVAL_SECONDS re-measures; the rest
    // extrapolate. Timed over a whole interval so the answer is what a
    // second of audio costs rather than what the lucky push costs.
    let pushes = (ESTIMATE_INTERVAL_SECONDS / hop).ceil() as u32;
    let start = std::time::Instant::now();
    for i in 0..pushes {
        tracker.push((i % 47) as f32 * 0.01);
    }
    eprintln!(
        "{pushes} pushes covering one estimate: {:.1} µs total, over {:.0} ms of audio",
        start.elapsed().as_secs_f64() * 1e6,
        pushes as f32 * hop * 1000.0
    );
}

// -- the window ---------------------------------------------------------

/// The window is exactly one octave wide, and that is the property the
/// whole design rests on: any tempo folds into it, and only one fold of it
/// does.
#[test]
fn the_window_is_one_octave_and_admits_exactly_one_fold() {
    for centre in [60.0_f32, 87.0, 120.0, 174.0, 200.0] {
        let window = tracking_window(centre);
        let ratio = window.end() / window.start();
        assert!(
            (ratio - 2.0).abs() < 1e-4,
            "the window around {centre} spans {ratio} octaves"
        );
        // Every tempo, not merely a few: each folds in, and its double and
        // its half do not.
        for step in 0..400 {
            let bpm = 20.0 * 1.01_f32.powi(step);
            let folded = fold(bpm, centre);
            assert!(
                *window.start() <= folded && folded <= *window.end(),
                "{bpm} folded to {folded}, outside {window:?}"
            );
            assert!(
                !window.contains(&(folded * 2.0)) && !window.contains(&(folded * 0.5)),
                "{window:?} holds both {folded} and one of its octaves"
            );
            // Folding is by powers of two and nothing else: the answer is
            // always the same tempo, an exact number of octaves away.
            let octaves = (folded / bpm).log2();
            assert!(
                (octaves - octaves.round()).abs() < 1e-5,
                "{bpm} folded to {folded}, which is {octaves} octaves away"
            );
        }
    }
}

/// A tempo already in the window is left exactly alone, folding is
/// idempotent — including on the boundary, where the answer itself is a
/// coin toss and only the stability matters — and the factor is always a
/// power of two.
#[test]
fn a_tempo_in_the_window_folds_to_itself_and_folding_again_changes_nothing() {
    assert_eq!(fold(128.0, 128.0), 128.0);
    assert_eq!(fold(174.0, 128.0), 174.0);
    assert_eq!(fold(96.0, 128.0), 96.0);
    // Two octaves either way, exactly.
    assert_eq!(fold(512.0, 128.0), 128.0);
    assert_eq!(fold(32.0, 128.0), 128.0);
    // On the boundary and either side of it, and everywhere else.
    let edge = 128.0 * std::f32::consts::SQRT_2;
    for bpm in [
        edge,
        edge - 0.01,
        edge + 0.01,
        60.0,
        200.0,
        87.0,
        349.0,
        // ...and a tempo that is not one lands in the window too, which is
        // the one thing a caller is allowed to assume.
        0.0,
        -12.0,
        f32::NAN,
        f32::INFINITY,
    ] {
        let once = fold(bpm, 128.0);
        assert_eq!(once, fold(once, 128.0), "{bpm} did not settle");
        assert!(tracking_window(128.0).contains(&once), "{bpm} → {once}");
    }
}

/// A centre outside the searchable range is clamped rather than believed,
/// and a nonsense one does not poison every fold after it. A tap at 400 bpm
/// is a grid nothing here can measure; a window centred there would answer
/// with tempi no lag in the search could have produced.
#[test]
fn a_centre_outside_the_range_is_clamped_and_a_nonsense_one_is_ignored() {
    assert_eq!(tracking_window(400.0), tracking_window(*BPM_RANGE.end()));
    assert_eq!(tracking_window(10.0), tracking_window(*BPM_RANGE.start()));
    for centre in [400.0_f32, 10.0, 0.0, -5.0, f32::NAN, f32::INFINITY] {
        let window = tracking_window(centre);
        assert!(
            *window.start() >= *BPM_RANGE.start() / 2.0 && *window.end() <= *BPM_RANGE.end() * 2.0,
            "a centre of {centre} gave the window {window:?}"
        );
        // Whatever the centre was taken to mean, the fold means the same.
        let folded = fold(133.0, centre);
        assert!(
            window.contains(&folded),
            "133 → {folded}, outside {window:?}"
        );
    }
    assert_eq!(
        Tracker::new(0.01, 0.02, 400.0).centre_bpm(),
        *BPM_RANGE.end()
    );
}

// -- the estimate -------------------------------------------------------

#[test]
fn a_click_train_gives_its_own_tempo() {
    for bpm in [90.0_f32, 128.0, 174.0] {
        let estimate = track(&clicks(bpm, 12.0, 0.8), bpm).estimate();
        assert!(
            (estimate.bpm - bpm).abs() < bpm * 0.01,
            "a {bpm} bpm train read {} bpm",
            estimate.bpm
        );
        assert!(
            estimate.confidence > 0.5,
            "a clean click train read confidence {}",
            estimate.confidence
        );
    }
}

/// Verifies tempo estimation accuracy across multiple tempos (including fast tempos up to 196 BPM).
#[test]
fn the_tempo_is_accurate_enough_to_run_a_grid_off() {
    for bpm in [128.0_f32, 174.0, 196.0] {
        for seconds in [12.0_f32, 16.0] {
            let estimate = track(&clicks(bpm, seconds, 0.8), bpm).estimate();
            let error = (estimate.bpm - bpm).abs() / bpm;
            assert!(
                error < 0.001,
                "a {bpm} bpm train over {seconds}s read {} — {}% out, which drifts a \
                     beat in {} seconds",
                estimate.bpm,
                error * 100.0,
                (1.0 / (error * bpm / 60.0)) as u32
            );
        }
    }
}

/// The phase points at the beat, not merely at a constant offset from it.
#[test]
fn the_phase_lands_on_the_beat() {
    // Clicks at sample 0 and every period after, so a beat lands exactly on
    // stream time k * 60/bpm.
    let bpm = 120.0_f32;
    let estimate = track(&clicks(bpm, 12.0, 0.8), bpm).estimate();

    // Where the estimate thinks the most recent beat was, in stream time.
    let beats_per_second = estimate.bpm as f64 / 60.0;
    let last_beat = estimate.at - estimate.phase as f64 / beats_per_second;
    // ...against where the beats actually are.
    let period = 60.0 / bpm as f64;
    let off = (last_beat / period).fract();
    let off = if off > 0.5 { off - 1.0 } else { off };
    assert!(
        off.abs() < 0.05,
        "the beat was placed {off} of a beat away from where it is"
    );
}

/// Silence has no tempo and says so, rather than reporting a confident
/// zero or the last thing it saw.
#[test]
fn silence_has_no_tempo_and_no_confidence() {
    let tracker = track(&vec![0.0; RATE as usize * 10], 120.0);
    assert_eq!(tracker.estimate().confidence, 0.0);
}

/// Something with no beat in it must not read as a confident tempo. A
/// sustained tone has plenty of energy and no pulses at all.
#[test]
fn a_sustained_tone_is_not_a_tempo() {
    let samples: Vec<f32> = (0..RATE as usize * 10)
        .map(|n| 0.5 * (TAU * 440.0 * n as f32 / RATE as f32).sin())
        .collect();
    let estimate = track(&samples, 120.0).estimate();
    assert!(
        estimate.confidence < 0.3,
        "a sustained tone read confidence {}",
        estimate.confidence
    );
}

/// The same novelty gives the same estimate. Replay depends on the `tempo`
/// record rather than on this, but an estimator whose answer depended on
/// anything else would be untestable as well as unrecordable.
#[test]
fn the_same_input_produces_the_same_estimate() {
    let samples = clicks(128.0, 14.0, 0.8);
    let once = track(&samples, 120.0).estimate();
    let twice = track(&samples, 120.0).estimate();
    assert_eq!(once, twice);
    // And the centre is part of the input, not of the history: a tracker
    // told the same things in the same order lands in the same place.
    let moved = {
        let mut tracker = track(&samples, 120.0);
        tracker.set_centre_bpm(120.0);
        tracker.estimate()
    };
    assert_eq!(once, moved);
}

// -- the octave ---------------------------------------------------------

/// Verifies that tempos across the full range resolve correctly without sub-octave folding.
#[test]
fn every_tempo_in_the_range_reads_itself_and_not_its_half() {
    let mut wrong = Vec::new();
    for bpm in [
        62.0_f32, 70.0, 85.0, 100.0, 110.0, 120.0, 128.0, 140.0, 150.0, 160.0, 165.0, 170.0, 174.0,
        180.0, 185.0, 190.0, 196.0,
    ] {
        for seconds in [12.0_f32, 16.0] {
            let samples = clicks(bpm, seconds, 0.8);
            // The true tempo, and an operator 30% out either way — still
            // inside the window, which reaches ±41%.
            for centre in [bpm, bpm * 1.3, bpm / 1.3] {
                let estimate = track(&samples, centre).estimate();
                if (estimate.bpm - bpm).abs() > bpm * 0.02 || estimate.confidence < 0.5 {
                    wrong.push((bpm, seconds, centre, estimate.bpm, estimate.confidence));
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "click trains misread (bpm, window, centre, read, confidence): {wrong:?}"
    );
}

/// Eighth notes with the beat on every other one: the correlation peak is
/// at the beat, the subdivision is inside the window too, and the estimate
/// has to come back with the beat.
#[test]
fn a_subdivided_pattern_reads_the_beat_and_not_the_subdivision() {
    // 128 bpm beats, with a weak click between each pair.
    let bpm = 128.0_f32;
    let period = (60.0 / bpm * RATE as f32) as usize;
    let len = RATE as usize * 14;
    let mut samples = vec![0.0f32; len];
    let mut at = 0;
    let mut strong = true;
    while at < len {
        let amplitude = if strong { 0.9 } else { 0.15 };
        for n in 0..64.min(len - at) {
            samples[at + n] = amplitude * (1.0 - n as f32 / 64.0);
        }
        at += period / 2;
        strong = !strong;
    }
    let estimate = track(&samples, bpm).estimate();
    assert!(
        (estimate.bpm - bpm).abs() < 4.0,
        "a subdivided {bpm} bpm pattern read {} bpm",
        estimate.bpm
    );
}

/// The other direction: every pulse is a beat, and a grid at half tempo
/// would fit them all. It must not settle there.
#[test]
fn an_even_click_train_is_not_read_at_half_tempo() {
    for bpm in [120.0_f32, 140.0] {
        let estimate = track(&clicks(bpm, 14.0, 0.8), bpm).estimate();
        assert!(
            (estimate.bpm - bpm / 2.0).abs() > 5.0,
            "a {bpm} bpm train settled at half tempo: {}",
            estimate.bpm
        );
        assert!(
            (estimate.bpm - bpm).abs() < bpm * 0.02,
            "a {bpm} bpm train read {}",
            estimate.bpm
        );
    }
}

/// A kick with a quiet hat between every pair of beats. The hat is a beat
/// *subdivision*, not a beat, so this must read the kick's tempo — and the
/// wrong answer here is the expensive one: it is a confident double, not a
/// shrug. It read 180 bpm at confidence 1.0 before the fold replaced the
/// check that was supposed to prevent exactly that.
///
/// Several window lengths and several hat levels, because the two
/// interleaved sets of grid points used to be distinguishable only by which
/// one the novelty was stronger at, and which of them a walk reached first
/// depended on where the window happened to end.
#[test]
fn a_quiet_offbeat_is_a_subdivision_and_not_a_doubled_tempo() {
    let bpm = 90.0_f32;
    let mut wrong = Vec::new();
    for seconds in [11.0_f32, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0] {
        for hat in [0.15_f32, 0.3, 0.45] {
            let estimate = track(&kick_and_hat(bpm, seconds, hat), bpm).estimate();
            if (estimate.bpm - bpm).abs() > bpm * 0.05 {
                wrong.push((seconds, hat, estimate.bpm, estimate.confidence));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "a quiet offbeat moved the octave (window, hat, read, confidence): {wrong:?}"
    );
}

/// A kick every beat with a quieter hat halfway between.
fn kick_and_hat(bpm: f32, seconds: f32, hat: f32) -> Vec<f32> {
    let period = (60.0 / bpm * RATE as f32) as usize;
    let len = (seconds * RATE as f32) as usize;
    let mut samples = vec![0.0f32; len];
    let burst = |s: &mut Vec<f32>, at: usize, amplitude: f32| {
        for n in 0..64.min(s.len().saturating_sub(at)) {
            s[at + n] = amplitude * (1.0 - n as f32 / 64.0);
        }
    };
    let mut at = 0;
    while at < len {
        burst(&mut samples, at, 0.9);
        if at + period / 2 < len {
            burst(&mut samples, at + period / 2, hat);
        }
        at += period;
    }
    samples
}

/// Verifies that tracking windows centred an octave away consistently lock to the octave-offset grid.
#[test]
fn a_window_centred_an_octave_off_locks_an_octave_off() {
    let estimate = track(&clicks(174.0, 14.0, 0.8), 87.0).estimate();
    assert!(
        (estimate.bpm - 87.0).abs() < 87.0 * 0.02,
        "a 174 bpm train under an 87 bpm window read {} bpm",
        estimate.bpm
    );
    // And it is believed, which is the part that used to be wrong: an
    // octave error collapsed the confidence and read as an absent beat.
    assert!(
        estimate.confidence > crate::lock::GATE_CONFIDENCE,
        "the octave-low grid read confidence {}, so it would never lock",
        estimate.confidence
    );
    // The other direction: a window an octave above the music.
    let estimate = track(&clicks(90.0, 14.0, 0.8), 180.0).estimate();
    assert!(
        (estimate.bpm - 180.0).abs() < 180.0 * 0.02,
        "a 90 bpm train under a 180 bpm window read {} bpm",
        estimate.bpm
    );
    assert!(
        estimate.confidence > crate::lock::GATE_CONFIDENCE,
        "the octave-high grid read confidence {}, so it would never lock",
        estimate.confidence
    );
}

/// The hint, which is the only thing left of the old automatic decision.
/// It fires when the grid really is at half the music's tempo, and stays
/// quiet on a kick with a quiet hat — which is the pattern the old
/// automatic double turned into a confident wrong tempo.
#[test]
fn the_half_tempo_hint_fires_on_a_grid_an_octave_low_and_not_on_an_offbeat() {
    let low = track(&clicks(174.0, 14.0, 0.8), 87.0).estimate();
    assert!(
        low.half_tempo_hint,
        "a grid at half the music's tempo did not hint (read {} bpm, c{})",
        low.bpm, low.confidence
    );
    let right = track(&clicks(174.0, 14.0, 0.8), 174.0).estimate();
    assert!(
        !right.half_tempo_hint,
        "a correct grid hinted at half tempo"
    );
    for hat in [0.15_f32, 0.3, 0.45] {
        let estimate = track(&kick_and_hat(90.0, 14.0, hat), 90.0).estimate();
        assert!(
            !estimate.half_tempo_hint,
            "a quiet {hat} offbeat hinted at half tempo"
        );
    }
}

/// Verifies that moving the tracking window centre dynamically tracks accelerating tempo across octave boundaries.
#[test]
fn a_tempo_that_drifts_across_the_boundary_is_followed_when_the_centre_moves() {
    let start = 130.0_f32;
    let end = 195.0_f32;
    let samples = accelerating(start, end, 40.0);

    let mut analyzer = Analyzer::new(RATE);
    let mut moving = Tracker::new(analyzer.hop_seconds(), analyzer.window_lag(), start);
    let mut fixed = Tracker::new(analyzer.hop_seconds(), analyzer.window_lag(), start);
    let mut block = vec![0.0f32; BLOCK];
    let mut at = 0;
    while at + BLOCK <= samples.len() {
        block.copy_from_slice(&samples[at..at + BLOCK]);
        let novelty = analyzer.analyze(&block).novelty;
        moving.push(novelty);
        fixed.push(novelty);
        // What a grid does: it is at the tempo it was last told about.
        let estimate = moving.estimate();
        if estimate.confidence > crate::lock::GATE_CONFIDENCE {
            moving.set_centre_bpm(estimate.bpm);
        }
        at += HOP;
    }

    // The boundary the music crossed: the static window's top edge.
    assert!(
        end > *tracking_window(start).end(),
        "the drift never left the static window, so this proves nothing"
    );
    let followed = moving.estimate();
    assert!(
        (followed.bpm - end).abs() < end * 0.04,
        "the moving window ended at {} bpm and the music at {end}",
        followed.bpm
    );
    // ...and the same novelty under a window that never moved is exactly
    // the octave error this design accepts, which is what keeps the
    // assertion above from passing against a tracker that ignores the
    // centre entirely.
    let stuck = fixed.estimate();
    assert!(
        (stuck.bpm - end / 2.0).abs() < end * 0.04,
        "the static window ended at {} bpm, neither the music nor its half",
        stuck.bpm
    );
}

/// A click train whose tempo rises linearly from `from` to `to`.
fn accelerating(from: f32, to: f32, seconds: f32) -> Vec<f32> {
    let len = (seconds * RATE as f32) as usize;
    let mut samples = vec![0.0f32; len];
    let mut at = 0usize;
    while at < len {
        for n in 0..64.min(len - at) {
            samples[at + n] = 0.8 * (1.0 - n as f32 / 64.0);
        }
        let bpm = from + (to - from) * (at as f32 / len as f32);
        at += (60.0 / bpm * RATE as f32) as usize;
    }
    samples
}

// -- extrapolation ------------------------------------------------------

/// The grid runs on its own between estimates. This is the mechanism the
/// brief calls "predict, do not chase", so it is asserted directly.
#[test]
fn a_grid_extrapolates_forward_at_its_own_tempo() {
    let estimate = Estimate {
        bpm: 120.0,
        phase: 0.25,
        confidence: 1.0,
        at: 10.0,
        revision: 1,
        half_tempo_hint: false,
    };
    // Half a second at 120 bpm is exactly one beat.
    assert!((estimate.phase_at(10.5) - 0.25).abs() < 1e-6);
    // A quarter second is half a beat.
    assert!((estimate.phase_at(10.25) - 0.75).abs() < 1e-6);
    // And it wraps rather than running past 1.0.
    assert!((0.0..1.0).contains(&estimate.phase_at(17.3)));
}
