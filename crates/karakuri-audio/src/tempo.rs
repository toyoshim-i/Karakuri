//! Tempo and beat-phase estimation from the analyser's novelty curve.
//!
//! **This estimates; it does not decide.** What the oscillator is told is
//! [`crate::lock`]'s business, and the split matters: an estimator that also
//! steered would have to be tuned for agility, and the thing being built here
//! wants to be tuned for *accuracy while the tempo holds*, which is where
//! essentially all of a set is spent.
//!
//! Pure, and no clock: a [`Tracker`] is a function of the novelty samples
//! pushed into it. Its notion of time is **stream seconds** — samples consumed
//! divided by the sample rate — so a test can drive it at any speed and a
//! device can drive it at one.
//!
//! ## How the estimate is made
//!
//! One window of novelty (about [`WINDOW_SECONDS`] of it), and two questions
//! asked of it:
//!
//! - **Period**, by autocorrelation over the lags inside [`BPM_RANGE`],
//!   weighted by a log-normal prior around [`PRIOR_BPM`]. The prior is not a
//!   thumb on the scale for a favourite tempo; it is the only thing that makes
//!   the octave choice well posed at all, since a grid at half the true tempo
//!   correlates about as well as the true one.
//! - **Phase**, by a single DFT bin at the beat frequency. That is the whole
//!   estimate rather than a search: `arg(S)` of `Σ novelty[n]·e^{-2πin/p}` is
//!   where the pulses sit, and `|S|` relative to the novelty's own mass says
//!   how pulse-like the window was, which is most of the confidence.
//!
//! ## The octave problem, handled rather than hoped about
//!
//! A grid at half the true tempo fits every beat and is wrong by no local
//! measure, so autocorrelation alone cannot settle it. Two explicit checks run
//! after the peak is picked, in this order:
//!
//! - **Halve** — if the novelty at odd grid points is much weaker than at even
//!   ones ([`ALTERNATION`]), the real beat is every *other* pulse: a bar of
//!   eighth notes with the beat on the downbeats reads as double tempo
//!   otherwise.
//! - **Double** — if the novelty *between* grid points is nearly as strong as
//!   on them ([`INTERLEAVE`]), there is a beat there too and the grid is at
//!   half tempo.
//!
//! Both are ratios of measured novelty at named positions, so both are
//! testable against a synthesised pattern rather than against a recording
//! somebody has to trust.

use std::f32::consts::TAU;

/// How much novelty history an estimate is made from. Long, because tempo
/// accuracy is the goal and a longer window resolves the period more finely:
/// eight seconds is sixteen beats at 120 bpm, enough that a period error of
/// half a percent is visible in the correlation. It is also why the estimate
/// is slow to react to a change, which is the trade this design is choosing on
/// purpose — a tempo step happens a few times an hour, inside a blend where
/// there are two tempi in the room and no correct answer.
pub const WINDOW_SECONDS: f32 = 8.0;

/// The tempo range considered. Wider than a DJ set needs at both ends, so that
/// the octave checks below have somewhere to move the answer to.
pub const BPM_RANGE: std::ops::RangeInclusive<f32> = 60.0..=200.0;

/// The centre of the octave prior. Nothing about a specific genre — it is the
/// middle of `BPM_RANGE` in log space, which is what makes the prior symmetric
/// between a candidate and its double.
pub const PRIOR_BPM: f32 = 110.0;

/// Width of the prior, in octaves. At 0.65, a candidate one octave out is
/// weighted about a third as much: enough to break a tie, not enough to
/// override a clear peak.
const PRIOR_OCTAVES: f32 = 0.65;

/// Below this ratio of odd-grid to even-grid novelty, the beat is every other
/// pulse and the estimate halves.
const ALTERNATION: f32 = 0.5;

/// Above this ratio of between-grid to on-grid novelty, there are beats between
/// the beats and the estimate doubles.
const INTERLEAVE: f32 = 0.8;

/// The normalised autocorrelation at which a period is believed completely.
/// Half: a click train reaches about 0.6 and music with a mixed kit reaches
/// less, so demanding more would mean never being confident about real
/// material.
const PERIODICITY_FULL: f32 = 0.5;

/// How often an estimate is recomputed, in seconds of stream time. The window
/// is eight seconds long, so recomputing faster than this changes almost
/// nothing and costs an autocorrelation.
const ESTIMATE_INTERVAL_SECONDS: f32 = 0.25;

/// What the tracker believes about the music, and how much.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Estimate {
    pub bpm: f32,
    /// Beat phase at [`Estimate::at`]: 0.0 is on the beat, rising to 1.0 at the
    /// next one.
    pub phase: f32,
    /// `[0, 1]`. Zero means "no idea", and everything downstream is required to
    /// treat that as "leave the oscillator alone" rather than "the tempo is
    /// zero".
    pub confidence: f32,
    /// The instant `phase` describes, in **stream seconds** — the middle of the
    /// newest analysis window, which is the newest moment the tracker can say
    /// anything about. Everything that turns this into a correction has to
    /// account for how old it is by then; see [`crate::lock`].
    pub at: f64,
    /// How many times the window has been *re-measured*. Between measurements
    /// the estimate is extrapolated forward, which changes `at` and `phase`
    /// without being new evidence — and a controller that counted extrapolated
    /// estimates as evidence would reach any threshold in a few frames. This is
    /// what lets it count opinions rather than frames.
    pub revision: u64,
}

impl Estimate {
    /// An estimate that says nothing. Not an `Option`, for the same reason the
    /// signal bus is not: a consumer branching on presence is a consumer that
    /// will forget one of the two branches.
    pub fn unknown(at: f64) -> Estimate {
        Estimate {
            bpm: 0.0,
            phase: 0.0,
            confidence: 0.0,
            at,
            revision: 0,
        }
    }

    /// Where this grid says the phase will be `ahead` seconds after the instant
    /// it describes.
    ///
    /// This is the form a correction uses, and the reason it takes a duration
    /// rather than an instant is that the duration is the sum of things the
    /// caller knows and this does not: how old the estimate is by now, and how
    /// long it will be until what is drawn now is light. See [`crate::lock`].
    pub fn phase_ahead(&self, ahead: f32) -> f32 {
        let beats = ahead as f64 * self.bpm as f64 / 60.0;
        (self.phase as f64 + beats).rem_euclid(1.0) as f32
    }

    /// Where the beat grid this estimate describes says the phase will be at
    /// stream time `when` — the extrapolation the whole design rests on. While
    /// the tempo holds this is simply correct, and no per-beat chasing is
    /// needed to keep it so.
    pub fn phase_at(&self, when: f64) -> f32 {
        let beats = (when - self.at) * self.bpm as f64 / 60.0;
        (self.phase as f64 + beats).rem_euclid(1.0) as f32
    }
}

/// Estimates tempo and beat phase from a novelty curve.
pub struct Tracker {
    hop_seconds: f32,
    /// Novelty, oldest first, capped at [`WINDOW_SECONDS`]. A `VecDeque` would
    /// be the obvious shape; this is a ring in a `Vec` because the whole window
    /// is walked in order on every estimate and a contiguous copy is what the
    /// autocorrelation wants.
    history: Vec<f32>,
    write: usize,
    filled: usize,
    /// Scratch for the linearised window. Allocated once: an estimate can be
    /// computed on the audio thread.
    window: Vec<f32>,
    /// Stream seconds at the centre of the newest window pushed.
    at: f64,
    /// Stream seconds when the cached estimate was made.
    estimated_at: f64,
    estimate: Estimate,
    revision: u64,
    lag_range: (usize, usize),
}

impl Tracker {
    pub fn new(hop_seconds: f32, window_lag: f32) -> Tracker {
        let hop_seconds = hop_seconds.max(1e-6);
        let len = (WINDOW_SECONDS / hop_seconds).ceil() as usize;
        // A lag is a whole number of hops, so the tempo resolution at the fast
        // end is coarser than at the slow end — 200 bpm is 14 hops at 48 kHz
        // and one hop either way is 7%. The refinement below interpolates the
        // correlation peak for exactly this reason.
        let slowest = (60.0 / BPM_RANGE.start() / hop_seconds).round() as usize;
        let fastest = (60.0 / BPM_RANGE.end() / hop_seconds).round() as usize;
        Tracker {
            hop_seconds,
            history: vec![0.0; len],
            write: 0,
            filled: 0,
            window: vec![0.0; len],
            // `at` is the centre of the newest window pushed, in stream
            // seconds. The first block covers samples `0..BLOCK`, so its centre
            // is one `window_lag` in; starting a hop before that means the
            // first `push` lands exactly there.
            at: f64::from(window_lag - hop_seconds),
            estimated_at: f64::NEG_INFINITY,
            estimate: Estimate::unknown(0.0),
            revision: 0,
            lag_range: (fastest.max(2), slowest.min(len / 2)),
        }
    }

    /// Push one analysis block's novelty. One call per [`crate::analysis::HOP`]
    /// samples, in order.
    pub fn push(&mut self, novelty: f32) {
        self.history[self.write] = if novelty.is_finite() {
            novelty.max(0.0)
        } else {
            0.0
        };
        self.write = (self.write + 1) % self.history.len();
        self.filled = (self.filled + 1).min(self.history.len());
        self.at += f64::from(self.hop_seconds);

        if self.at - self.estimated_at >= f64::from(ESTIMATE_INTERVAL_SECONDS) {
            self.estimated_at = self.at;
            self.revision += 1;
            self.estimate = self.estimate_now();
        } else {
            // Between recomputations the grid is *extrapolated*, not held: the
            // phase this estimate describes moves on at the tempo it found.
            // That is the whole "predict, do not chase" position, and it is why
            // an estimate carries the instant it refers to.
            //
            // Phase first, then the instant it refers to: `phase_at` measures
            // from `estimate.at`, so moving that first would make every
            // extrapolation a no-op and leave the estimate labelled fresh while
            // standing still — worth a quarter of a beat at 120 bpm, and
            // invisible except as a grid that is mysteriously late.
            self.estimate.phase = self.estimate.phase_at(self.at);
            self.estimate.at = self.at;
        }
    }

    /// The current estimate. Cheap — the work happens in [`Tracker::push`].
    pub fn estimate(&self) -> Estimate {
        self.estimate
    }

    /// Stream seconds at the centre of the newest analysis window.
    pub fn at(&self) -> f64 {
        self.at
    }

    /// "No opinion", but still a new opinion: it carries the revision, so a
    /// controller sees that the tracker has spoken and said it does not know,
    /// rather than seeing the previous estimate stand.
    fn nothing(&self) -> Estimate {
        Estimate {
            revision: self.revision,
            ..Estimate::unknown(self.at)
        }
    }

    fn estimate_now(&mut self) -> Estimate {
        let len = self.history.len();
        // Half a window is enough to say something; less is a tempo guessed
        // from two beats.
        if self.filled < len / 2 {
            return self.nothing();
        }

        // Linearise oldest-first, and remove the mean: autocorrelation of a
        // curve with a large DC term finds the DC term.
        let count = self.filled;
        let start = (self.write + len - count) % len;
        let mut sum = 0.0;
        for i in 0..count {
            let v = self.history[(start + i) % len];
            self.window[i] = v;
            sum += v;
        }
        let mean = sum / count as f32;
        if sum <= 0.0 {
            // Silence has no tempo, and saying so is not the same as saying
            // zero: confidence 0.0 leaves the oscillator free-running.
            return self.nothing();
        }
        for v in &mut self.window[..count] {
            *v -= mean;
        }
        let window = &self.window[..count];

        let (lo, hi) = self.lag_range;
        if hi <= lo {
            return self.nothing();
        }

        let mut best = (0usize, f32::NEG_INFINITY);
        for lag in lo..=hi {
            let bpm = 60.0 / (lag as f32 * self.hop_seconds);
            let score = autocorrelation(window, lag) * prior(bpm);
            if score > best.1 {
                best = (lag, score);
            }
        }
        let (lag, peak) = best;
        if peak <= 0.0 {
            return self.nothing();
        }

        // Sub-hop refinement: fit a parabola through the correlation at the
        // peak and its neighbours. Without it the tempo is quantised to whole
        // hops — 1.5 bpm at 128 bpm and 48 kHz — and a grid running 1% fast
        // drifts a whole beat inside ten seconds, which is exactly the
        // steady-state error this design exists to avoid.
        let refined = refine(
            autocorrelation(window, lag.saturating_sub(1)),
            autocorrelation(window, lag),
            autocorrelation(window, lag + 1),
        );
        let period_hops = lag as f32 + refined;

        let (period_hops, phase, pulse) = self.settle_octave(window, period_hops);
        let bpm = 60.0 / (period_hops * self.hop_seconds);

        // Two things have to hold for an estimate to be worth acting on: the
        // novelty at one period apart has to actually resemble itself (there is
        // *a* period), and the novelty has to be concentrated at that period's
        // grid points (it is *this* period). Multiplying them means a failure
        // of either sinks the estimate, which is the conservative direction — a
        // confident wrong tempo is the only failure here that shows on stage.
        //
        // Normalised against the novelty's own variance rather than against the
        // mean score across lags: the window has had its mean removed, so the
        // mean score is approximately zero and dividing by it says nothing at
        // all.
        let variance = autocorrelation(window, 0);
        let periodicity = if variance > 0.0 {
            (autocorrelation(window, lag) / variance / PERIODICITY_FULL).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let confidence = (periodicity * pulse).clamp(0.0, 1.0);

        Estimate {
            bpm: bpm.clamp(*BPM_RANGE.start(), *BPM_RANGE.end()),
            phase,
            confidence,
            at: self.at,
            revision: self.revision,
        }
    }

    /// Decide between a period, its half, and its double, and return the
    /// settled period with the phase and pulse-concentration that go with it.
    fn settle_octave(&self, window: &[f32], period: f32) -> (f32, f32, f32) {
        let (phase, pulse) = pulse_phase(window, period);

        // Halve: are the odd grid points much weaker than the even ones? Then
        // what was found is the subdivision and the beat is every other one.
        let even = grid_mean(window, period, phase, 0.0, 2.0);
        let odd = grid_mean(window, period, phase, 1.0, 2.0);
        if even > 0.0 && odd < even * ALTERNATION {
            let doubled = period * 2.0;
            if 60.0 / (doubled * self.hop_seconds) >= *BPM_RANGE.start() {
                let (phase, pulse) = pulse_phase(window, doubled);
                return (doubled, phase, pulse);
            }
        }

        // Double: is there nearly as much novelty *between* the grid points as
        // on them? Then there is a beat there too and this grid is at half
        // tempo. Both means are taken on the **same** grid, in units of the
        // same period, one of them shifted half a beat — measuring the halves
        // against a grid of `period / 2` instead would need the phase converted
        // into those units, and getting that wrong silently samples the wrong
        // positions.
        //
        // The *phase* for this pair comes from the half period, though, and it
        // has to: `phase` above is a DFT bin at `1 / period`, and if `period` is
        // twice the real pulse spacing — exactly the case this check exists to
        // catch — consecutive pulses land half a turn apart in that bin and
        // cancel. The magnitude goes to nothing, `atan2` of nothing is
        // arbitrary, and both grid means then sample troughs and come back
        // zero, which switches the check off precisely when it is needed. The
        // bin at `2 / period` has no such degeneracy in either direction, so the
        // fine grid is where the phase is measured; halving it converts it back
        // into units of `period`, and both means are still taken on one grid.
        //
        // Which of the two interleaved sets holds the pulses is then a parity
        // the fine grid does not fix — it anchors on *a* pulse and says nothing
        // about whether that one is a downbeat. So they are sorted rather than
        // named: the question is whether the weaker set is nearly as strong as
        // the stronger, which is parity-free and is what the ratio meant all
        // along.
        let (fine_phase, _) = pulse_phase(window, period / 2.0);
        let grid = fine_phase * 0.5;
        let first = grid_mean(window, period, grid, 0.0, 1.0);
        let second = grid_mean(window, period, grid, 0.5, 1.0);
        let (on, between) = if first >= second {
            (first, second)
        } else {
            (second, first)
        };
        if on > 0.0 && between > on * INTERLEAVE {
            let halved = period / 2.0;
            if 60.0 / (halved * self.hop_seconds) <= *BPM_RANGE.end() {
                let (phase, pulse) = pulse_phase(window, halved);
                return (halved, phase, pulse);
            }
        }

        (period, phase, pulse)
    }
}

/// Unnormalised autocorrelation at `lag`, divided by the overlap so that long
/// lags are not penalised for having fewer terms to add.
fn autocorrelation(window: &[f32], lag: usize) -> f32 {
    if lag >= window.len() {
        return 0.0;
    }
    let overlap = window.len() - lag;
    let sum: f32 = (0..overlap).map(|i| window[i] * window[i + lag]).sum();
    sum / overlap as f32
}

/// The offset of a parabola's vertex through three equally spaced samples, in
/// `[-0.5, 0.5]`.
fn refine(before: f32, at: f32, after: f32) -> f32 {
    let denominator = before - 2.0 * at + after;
    if denominator.abs() < f32::EPSILON {
        return 0.0;
    }
    (0.5 * (before - after) / denominator).clamp(-0.5, 0.5)
}

/// Beat phase at the newest sample, and how concentrated the novelty is at that
/// period — one DFT bin at the beat frequency.
///
/// The phase convention is the one everything downstream uses: 0.0 is on the
/// beat, and the value returned is the phase at the *newest* novelty sample,
/// because that is the only instant the caller knows a wall time for.
fn pulse_phase(window: &[f32], period: f32) -> (f32, f32) {
    if period <= 0.0 || window.is_empty() {
        return (0.0, 0.0);
    }
    let mut real = 0.0;
    let mut imaginary = 0.0;
    let mut mass = 0.0;
    for (n, &v) in window.iter().enumerate() {
        // Only the positive part carries pulses; the negative part is the mean
        // removal's shadow and would rotate the answer.
        let v = v.max(0.0);
        let angle = TAU * n as f32 / period;
        real += v * angle.cos();
        imaginary += v * angle.sin();
        mass += v;
    }
    if mass <= 0.0 {
        return (0.0, 0.0);
    }
    // `atan2(imaginary, real)` is the angle of the pulse train's first
    // component; dividing by TAU turns it into a position within the beat, and
    // the newest sample is at index len-1.
    let beat_at = imaginary.atan2(real) / TAU;
    let newest = (window.len() - 1) as f32 / period;
    let phase = (newest - beat_at).rem_euclid(1.0);
    // A perfect pulse train puts all its mass in one bin; white novelty
    // scatters it. Doubling because a real train's mass is spread over a few
    // hops, which costs about half the magnitude.
    let concentration = (2.0 * (real * real + imaginary * imaginary).sqrt() / mass).clamp(0.0, 1.0);
    (phase, concentration)
}

/// Mean novelty on a grid, walked back from the newest sample.
///
/// Everything is in units of one `period`: the newest grid point is `phase`
/// back from the newest sample, `start` shifts the whole walk, and `step` is
/// how far apart the points are. Both octave checks use this, in the same
/// units, so there is one place where a grid position can be got wrong.
fn grid_mean(window: &[f32], period: f32, phase: f32, start: f32, step: f32) -> f32 {
    if period < 2.0 || step <= 0.0 || window.is_empty() {
        return 0.0;
    }
    let newest = (window.len() - 1) as f32;
    let mut sum = 0.0;
    let mut count = 0u32;
    let mut k = 0.0;
    loop {
        let position = newest - (phase + start + k) * period;
        if position < 1.0 {
            break;
        }
        // The pulse is a few hops wide, and a beat period is not a whole
        // number of hops, so one beat lands centred in a hop and the next
        // straddles two. Summing the neighbourhood rather than taking its
        // strongest sample is what makes those two read the same: a straddled
        // pulse keeps its energy and only loses its peak, and a check built on
        // the peak would call every other beat weak on any tempo whose period
        // is not a whole number of hops — which is nearly all of them.
        let index = position.round() as usize;
        sum += window[index.saturating_sub(1)..(index + 2).min(window.len())]
            .iter()
            .map(|v| v.max(0.0))
            .sum::<f32>();
        count += 1;
        k += step;
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f32
    }
}

/// A log-normal weight around [`PRIOR_BPM`]. See the module doc: without it the
/// octave choice has no well-posed answer at all.
fn prior(bpm: f32) -> f32 {
    let octaves = (bpm / PRIOR_BPM).log2() / PRIOR_OCTAVES;
    (-0.5 * octaves * octaves).exp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{Analyzer, BLOCK, HOP};

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

    /// Analyse a signal and feed the tracker, exactly as the device does.
    fn track(samples: &[f32]) -> (Tracker, Analyzer) {
        let mut analyzer = Analyzer::new(RATE);
        let mut tracker = Tracker::new(analyzer.hop_seconds(), analyzer.window_lag());
        let mut block = vec![0.0f32; BLOCK];
        let mut at = 0;
        while at + BLOCK <= samples.len() {
            block.copy_from_slice(&samples[at..at + BLOCK]);
            tracker.push(analyzer.analyze(&block).novelty);
            at += HOP;
        }
        (tracker, analyzer)
    }

    /// What an estimate costs, on a host clock, in the audio callback where it
    /// runs. Ignored for the same reason as `analysis`'s: it is a measurement.
    /// `cargo test -p karakuri-audio --release -- --ignored --nocapture`.
    #[test]
    #[ignore = "a measurement, not an assertion"]
    fn what_one_estimate_costs() {
        let hop = HOP as f32 / RATE as f32;
        let mut tracker = Tracker::new(hop, BLOCK as f32 / 2.0 / RATE as f32);
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

    #[test]
    fn a_click_train_gives_its_own_tempo() {
        for bpm in [90.0_f32, 128.0, 174.0] {
            let (tracker, _) = track(&clicks(bpm, 12.0, 0.8));
            let estimate = tracker.estimate();
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

    /// The estimate is accurate enough to *extrapolate* from, which is the
    /// property the whole design leans on: a grid that is 1% off drifts a
    /// beat in ten seconds.
    #[test]
    fn the_tempo_is_accurate_enough_to_run_a_grid_off() {
        let (tracker, _) = track(&clicks(128.0, 16.0, 0.8));
        let estimate = tracker.estimate();
        let error = (estimate.bpm - 128.0).abs() / 128.0;
        assert!(
            error < 0.005,
            "the tempo was {}% out, which drifts a beat in {} seconds",
            error * 100.0,
            (1.0 / (error * 128.0 / 60.0)) as u32
        );
    }

    /// The phase points at the beat, not merely at a constant offset from it.
    #[test]
    fn the_phase_lands_on_the_beat() {
        // Clicks at sample 0 and every period after, so a beat lands exactly on
        // stream time k * 60/bpm.
        let bpm = 120.0_f32;
        let (tracker, _) = track(&clicks(bpm, 12.0, 0.8));
        let estimate = tracker.estimate();

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
        let (tracker, _) = track(&vec![0.0; RATE as usize * 10]);
        assert_eq!(tracker.estimate().confidence, 0.0);
    }

    /// Something with no beat in it must not read as a confident tempo. A
    /// sustained tone has plenty of energy and no pulses at all.
    #[test]
    fn a_sustained_tone_is_not_a_tempo() {
        let samples: Vec<f32> = (0..RATE as usize * 10)
            .map(|n| 0.5 * (TAU * 440.0 * n as f32 / RATE as f32).sin())
            .collect();
        let (tracker, _) = track(&samples);
        assert!(
            tracker.estimate().confidence < 0.3,
            "a sustained tone read confidence {}",
            tracker.estimate().confidence
        );
    }

    // -- the octave problem -------------------------------------------------

    /// Eighth notes with the beat on every other one: the correlation peak is
    /// at the subdivision, and the alternation check has to halve it.
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
        let (tracker, _) = track(&samples);
        let estimate = tracker.estimate();
        assert!(
            (estimate.bpm - bpm).abs() < 4.0,
            "a subdivided {bpm} bpm pattern read {} bpm",
            estimate.bpm
        );
    }

    /// The whole tempo range, not the three tempi that were convenient.
    ///
    /// The octave checks are where an estimator quietly stops working over part
    /// of its range, and the failure is silent: a half-tempo answer with the
    /// confidence collapsed reads as "no beat here" rather than as a bug, so
    /// nothing downstream complains — the grid simply never locks. Above about
    /// 160 bpm the correlation prior starts preferring the half-tempo candidate,
    /// which is precisely where the `double` check has to earn its place.
    ///
    /// Two windows per tempo, because the failure this catches was intermittent:
    /// the phase the check was reading was numerically arbitrary, so which
    /// window it was measured over decided the answer.
    #[test]
    fn every_tempo_in_the_range_reads_itself_and_not_its_half() {
        let mut wrong = Vec::new();
        for bpm in [
            62.0_f32, 70.0, 85.0, 100.0, 110.0, 120.0, 128.0, 140.0, 150.0, 160.0, 165.0, 170.0,
            174.0, 180.0, 185.0, 190.0, 196.0,
        ] {
            for seconds in [12.0_f32, 16.0] {
                let (tracker, _) = track(&clicks(bpm, seconds, 0.8));
                let estimate = tracker.estimate();
                if (estimate.bpm - bpm).abs() > bpm * 0.02 || estimate.confidence < 0.5 {
                    wrong.push((bpm, seconds, estimate.bpm, estimate.confidence));
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "click trains misread (bpm, window, read, confidence): {wrong:?}"
        );
    }

    /// The other direction: every pulse is a beat, and a grid at half tempo
    /// would fit them all. It must not settle there.
    #[test]
    fn an_even_click_train_is_not_read_at_half_tempo() {
        for bpm in [120.0_f32, 140.0] {
            let (tracker, _) = track(&clicks(bpm, 14.0, 0.8));
            let estimate = tracker.estimate();
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
    /// shrug.
    ///
    /// Several window lengths, because the two interleaved sets of grid points
    /// are only distinguishable by which one the novelty is stronger at. Which
    /// of them the walk reaches *first* depends on where the window happens to
    /// end, so a check that named them by position rather than by strength is
    /// right for half of all window lengths and confidently doubles for the
    /// other half.
    #[test]
    fn a_quiet_offbeat_is_a_subdivision_and_not_a_doubled_tempo() {
        let bpm = 90.0_f32;
        let period = (60.0 / bpm * RATE as f32) as usize;
        let mut wrong = Vec::new();
        for seconds in [11.0_f32, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0] {
            for hat in [0.15_f32, 0.3, 0.45] {
                let len = (seconds * RATE as f32) as usize;
                let mut samples = vec![0.0f32; len];
                let mut at = 0;
                let burst = |s: &mut Vec<f32>, at: usize, amplitude: f32| {
                    for n in 0..64.min(s.len().saturating_sub(at)) {
                        s[at + n] = amplitude * (1.0 - n as f32 / 64.0);
                    }
                };
                while at < len {
                    burst(&mut samples, at, 0.9);
                    if at + period / 2 < len {
                        burst(&mut samples, at + period / 2, hat);
                    }
                    at += period;
                }
                let (tracker, _) = track(&samples);
                let estimate = tracker.estimate();
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
        };
        // Half a second at 120 bpm is exactly one beat.
        assert!((estimate.phase_at(10.5) - 0.25).abs() < 1e-6);
        // A quarter second is half a beat.
        assert!((estimate.phase_at(10.25) - 0.75).abs() < 1e-6);
        // And it wraps rather than running past 1.0.
        assert!((0.0..1.0).contains(&estimate.phase_at(17.3)));
    }
}
