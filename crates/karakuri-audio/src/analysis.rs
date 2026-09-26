//! Audio block spectral analysis and feature extraction.
//! Transforms PCM sample blocks into frequency band energies, RMS levels,
//! and spectral flux onsets with standardized decibel scaling.

use std::sync::Arc;

use karakuri_signal::measured::{AudioFrame, MAX_BANDS};
use rustfft::{num_complex::Complex, Fft, FftPlanner};

/// Samples per analysis block (2048 samples / ~43 ms at 48 kHz, providing ~23 Hz bin resolution).
pub const BLOCK: usize = 2048;

/// Hop size in samples between analysis blocks (512 samples / 10.7 ms at 48 kHz).
pub const HOP: usize = 512;

/// Below this, a level reads 0.0. See the module doc.
pub const FLOOR_DB: f32 = -60.0;

/// At and above this, a level reads 1.0. See the module doc.
pub const TOP_DB: f32 = -6.0;

/// The low edge of the lowest band, in Hz. Below a kick's fundamental and above
/// the DC and rumble that no band should be measuring.
const BAND_LOW_HZ: f32 = 40.0;

/// The high edge of the highest band, in Hz. Above this is air and hiss; a band
/// up there measures the room's noise floor rather than the music.
const BAND_HIGH_HZ: f32 = 16_000.0;

/// Minimum relative spectral flux increase required to detect a transient onset.
const ONSET_SENSITIVITY: f32 = 2.5;

/// An absolute floor under the threshold, in the same RMS units as a level.
/// Without it, silence has a mean flux of zero and *any* numerical wobble is
/// infinitely many times larger than usual. −60 dBFS of spectral change, the
/// same floor a level has.
const ONSET_FLOOR: f32 = 0.001;

/// Blocks that must pass before another onset can fire. Three hops is 32 ms at
/// 48 kHz — shorter than any two events a listener hears as separate, longer
/// than the two or three blocks one attack spreads across.
const ONSET_REFRACTORY: u32 = 3;

/// How long the running mean flux remembers, in seconds. Long, because it is a
/// threshold rather than a signal: a mean that chased the flux would rise
/// through a drum fill and stop detecting it.
const FLUX_MEMORY_SECONDS: f32 = 1.0;

/// Half-life of the onset envelope, in seconds. Long enough to survive being
/// sampled at any frame rate, short enough that two beats do not merge.
const ONSET_HALF_LIFE_SECONDS: f32 = 0.12;

/// One block's worth of analysis.
pub struct Analysis {
    /// The measured signals, at full confidence: a measurement happened. What
    /// happens to that confidence as this frame ages is
    /// [`crate::device::staleness`]'s business, because that is a question
    /// about elapsed real time.
    pub frame: AudioFrame,
    /// Spectral flux — how much the spectrum moved since the previous block,
    /// counting only the parts that got louder. The novelty function the tempo
    /// tracker runs on; exposed because a tracker wants the continuous curve,
    /// not the envelope the bus sees.
    pub novelty: f32,
    /// Whether this block was detected as an onset.
    pub onset: bool,
}

/// Turns blocks of mono samples into [`AudioFrame`]s.
pub struct Analyzer {
    sample_rate: f32,
    fft: Arc<dyn Fft<f32>>,
    /// Hann. Preallocated with everything else: the audio callback calls
    /// `analyze`, and an allocation there is a lock in disguise.
    window: Vec<f32>,
    spectrum: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    /// Per-bin RMS contribution, this block and the previous one.
    magnitude: Vec<f32>,
    previous: Vec<f32>,
    /// `[low, high)` bin index per band.
    bands: [(usize, usize); MAX_BANDS],
    flux_mean: f32,
    flux_memory: f32,
    envelope: f32,
    envelope_decay: f32,
    since_onset: u32,
}

impl Analyzer {
    /// Plans the transform and the band edges for one sample rate. Allocates;
    /// never on the audio thread.
    pub fn new(sample_rate: u32) -> Analyzer {
        let sample_rate = sample_rate.max(1) as f32;
        let fft = FftPlanner::new().plan_fft_forward(BLOCK);
        let scratch = vec![Complex::new(0.0, 0.0); fft.get_inplace_scratch_len()];
        let bins = BLOCK / 2 + 1;
        let hop_seconds = HOP as f32 / sample_rate;

        Analyzer {
            sample_rate,
            fft,
            window: (0..BLOCK).map(|n| hann(n, BLOCK)).collect(),
            spectrum: vec![Complex::new(0.0, 0.0); BLOCK],
            scratch,
            magnitude: vec![0.0; bins],
            previous: vec![0.0; bins],
            bands: band_bins(sample_rate),
            flux_mean: 0.0,
            // A first-order memory expressed as a time constant, so the
            // detector behaves the same at 44.1 and 96 kHz rather than being
            // tuned for one of them.
            flux_memory: 1.0 - (-hop_seconds / FLUX_MEMORY_SECONDS).exp(),
            envelope: 0.0,
            envelope_decay: 0.5f32.powf(hop_seconds / ONSET_HALF_LIFE_SECONDS),
            since_onset: ONSET_REFRACTORY,
        }
    }

    /// How many bands this analyser measures. The frames it produces answer
    /// `band0`…`band<count-1>` and nothing past that.
    pub fn band_count(&self) -> u8 {
        MAX_BANDS as u8
    }

    /// Seconds between blocks — the rate measurement arrives at.
    pub fn hop_seconds(&self) -> f32 {
        HOP as f32 / self.sample_rate
    }

    /// Returns the effective content latency in seconds (half the analysis window).
    pub fn window_lag(&self) -> f32 {
        BLOCK as f32 / 2.0 / self.sample_rate
    }

    /// Analyse one block. `block.len()` must be [`BLOCK`].
    pub fn analyze(&mut self, block: &[f32]) -> Analysis {
        assert_eq!(block.len(), BLOCK, "an analysis block is {BLOCK} samples");

        // Broadband level comes off the raw block, not the windowed one: a
        // window is there to make the *spectrum* well behaved, and shaping the
        // level with it would mean the same signal read differently depending
        // on where in the window it sat.
        let mean_square: f32 = block.iter().map(|s| s * s).sum::<f32>() / BLOCK as f32;
        let energy = level(mean_square.sqrt());

        for (i, sample) in block.iter().enumerate() {
            self.spectrum[i] = Complex::new(sample * self.window[i], 0.0);
        }
        self.fft
            .process_with_scratch(&mut self.spectrum, &mut self.scratch);

        // Parseval, with the window's power gain divided out and the one-sided
        // spectrum's missing half put back, so that `magnitude[k]` is the RMS
        // the signal in that bin contributes. That is what makes a band's level
        // and `energy` the same quantity on the same scale.
        let scale = (16.0 / (3.0 * BLOCK as f32 * BLOCK as f32)).sqrt();
        std::mem::swap(&mut self.magnitude, &mut self.previous);
        for (k, m) in self.magnitude.iter_mut().enumerate() {
            *m = self.spectrum[k].norm() * scale;
        }

        let mut bands = [0.0f32; MAX_BANDS];
        for (band, &(low, high)) in self.bands.iter().enumerate() {
            let power: f32 = self.magnitude[low..high].iter().map(|m| m * m).sum();
            bands[band] = level(power.sqrt());
        }

        // Only the parts that got *louder*: a note ending is not an onset, and
        // counting it as one puts a hit on every release.
        let novelty: f32 = self
            .magnitude
            .iter()
            .zip(&self.previous)
            .skip(1) // DC is not music, and it moves with any offset drift.
            .map(|(now, before)| (now - before).max(0.0))
            .sum();

        let onset = novelty > self.flux_mean * ONSET_SENSITIVITY + ONSET_FLOOR
            && self.since_onset >= ONSET_REFRACTORY;
        // After the comparison: a threshold that included the sample it is
        // judging would raise itself out of the way of exactly the events it
        // exists to catch.
        self.flux_mean += (novelty - self.flux_mean) * self.flux_memory;

        if onset {
            self.envelope = 1.0;
            self.since_onset = 0;
        } else {
            self.envelope *= self.envelope_decay;
            self.since_onset = self.since_onset.saturating_add(1);
        }

        Analysis {
            frame: AudioFrame {
                energy,
                onset: self.envelope,
                bands,
                band_count: MAX_BANDS as u8,
                confidence: 1.0,
            },
            novelty,
            onset,
        }
    }
}

fn hann(n: usize, len: usize) -> f32 {
    0.5 * (1.0 - (std::f32::consts::TAU * n as f32 / len as f32).cos())
}

/// An RMS amplitude as a `[0, 1]` level. See the module doc for the two ends.
pub fn level(rms: f32) -> f32 {
    if rms <= 0.0 || rms.is_nan() {
        // Silence is exactly zero rather than "very small", so that a bound
        // parameter sits exactly at the bottom of its range in a quiet room
        // instead of hovering. A NaN takes the same exit: one bad block must
        // not put a NaN in a uniform.
        return 0.0;
    }
    let db = 20.0 * rms.log10();
    ((db - FLOOR_DB) / (TOP_DB - FLOOR_DB)).clamp(0.0, 1.0)
}

/// Computes log-spaced frequency band edges across 8 bands spanning 40 Hz to 16 kHz.
///
/// If upper band frequencies exceed Nyquist, bands collapse gracefully to single top bins.
fn band_bins(sample_rate: f32) -> [(usize, usize); MAX_BANDS] {
    let bins = BLOCK / 2 + 1;
    let hz_per_bin = sample_rate / BLOCK as f32;
    let ratio = (BAND_HIGH_HZ / BAND_LOW_HZ).powf(1.0 / MAX_BANDS as f32);

    let mut edges = [(0usize, 0usize); MAX_BANDS];
    // Computed sequentially to guarantee non-overlapping bins and at least one bin per band.
    let mut low = ((BAND_LOW_HZ / hz_per_bin).round() as usize).clamp(1, bins - MAX_BANDS);
    for (band, edge) in edges.iter_mut().enumerate() {
        let high_hz = BAND_LOW_HZ * ratio.powi(band as i32 + 1);
        // At least one bin per band, so a band always measures *something*
        // rather than reading silence because its edges landed between bins —
        // and never more than leaves one bin for each band still to be placed.
        let ceiling = bins - (MAX_BANDS - band - 1);
        let high = ((high_hz / hz_per_bin).round() as usize).clamp(low + 1, ceiling);
        *edge = (low, high);
        low = high;
    }
    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    /// A sine at `hz`, amplitude `a`, continuing from sample `start`.
    fn tone(hz: f32, a: f32, start: usize, len: usize) -> Vec<f32> {
        (0..len)
            .map(|n| {
                let t = (start + n) as f32 / RATE as f32;
                a * (std::f32::consts::TAU * hz * t).sin()
            })
            .collect()
    }

    fn silence(len: usize) -> Vec<f32> {
        vec![0.0; len]
    }

    /// A deterministic broadband source. Not `rand`: a test that needs a crate
    /// to make noise is a test that cannot be read.
    fn noise(len: usize, seed: u32) -> Vec<f32> {
        let mut x = seed | 1;
        (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                (x as f32 / u32::MAX as f32) * 2.0 - 1.0
            })
            .collect()
    }

    /// Feed a signal block by block and keep the last analysis.
    fn run(analyzer: &mut Analyzer, samples: &[f32]) -> Analysis {
        let mut last = analyzer.analyze(&silence(BLOCK));
        for block in samples.chunks_exact(BLOCK) {
            last = analyzer.analyze(block);
        }
        last
    }

    fn loudest_band(frame: &AudioFrame) -> usize {
        (0..MAX_BANDS)
            .max_by(|a, b| frame.bands[*a].total_cmp(&frame.bands[*b]))
            .expect("a band")
    }

    // -- the spectrum -------------------------------------------------------

    /// A tone lands in the band that contains it, and in no other. One test per
    /// band, so an off-by-one in the edges cannot hide in the one band nobody
    /// checked.
    #[test]
    fn a_tone_lands_in_the_band_that_contains_its_frequency() {
        let ratio = (BAND_HIGH_HZ / BAND_LOW_HZ).powf(1.0 / MAX_BANDS as f32);
        for band in 0..MAX_BANDS {
            // The geometric middle of the band, so the answer does not depend
            // on which side of an edge a rounded bin fell.
            let hz = BAND_LOW_HZ * ratio.powf(band as f32 + 0.5);
            let mut analyzer = Analyzer::new(RATE);
            let frame = run(&mut analyzer, &tone(hz, 0.5, 0, BLOCK * 4)).frame;
            assert_eq!(
                loudest_band(&frame),
                band,
                "a {hz:.0} Hz tone should be loudest in band {band}, bands were {:?}",
                frame.bands
            );
            // And it is not merely the loudest: the neighbours are far below
            // it, or "loudest" would be a coin toss between two bands.
            for other in 0..MAX_BANDS {
                if other != band {
                    assert!(
                        frame.bands[other] < frame.bands[band] - 0.2,
                        "band {other} at {} is too close to band {band} at {}",
                        frame.bands[other],
                        frame.bands[band]
                    );
                }
            }
        }
    }

    /// Verifies that named acoustic frequency bands (sub, bass, mid, air) correspond
    /// to the expected spectral regions and typed AudioFrame accessors.
    #[test]
    fn semantic_spectral_bands_capture_target_frequencies() {
        let targets = [
            (60.0_f32, 0, "sub"),
            (120.0_f32, 1, "bass"),
            (260.0_f32, 2, "low_mid"),
            (550.0_f32, 3, "mid"),
            (1150.0_f32, 4, "high_mid"),
            (2500.0_f32, 5, "presence"),
            (5200.0_f32, 6, "brilliance"),
            (11000.0_f32, 7, "air"),
        ];

        for (hz, expected_band, name) in targets {
            let mut analyzer = Analyzer::new(RATE);
            let frame = run(&mut analyzer, &tone(hz, 0.5, 0, BLOCK * 4)).frame;
            assert_eq!(
                loudest_band(&frame),
                expected_band,
                "{hz} Hz ({name}) was expected in band {expected_band}, but loudest was {}",
                loudest_band(&frame)
            );

            // Accessors match band array
            assert_eq!(frame.sub(), frame.bands[0]);
            assert_eq!(frame.bass(), frame.bands[1]);
            assert_eq!(frame.low_mid(), frame.bands[2]);
            assert_eq!(frame.mid(), frame.bands[3]);
            assert_eq!(frame.high_mid(), frame.bands[4]);
            assert_eq!(frame.presence(), frame.bands[5]);
            assert_eq!(frame.brilliance(), frame.bands[6]);
            assert_eq!(frame.air(), frame.bands[7]);
        }
    }

    /// Broadband noise is in every band, which is the complement of the test
    /// above: a band structure that always answered "band 3" would pass one of
    /// them and not both.
    #[test]
    fn broadband_noise_spreads_across_every_band() {
        let mut analyzer = Analyzer::new(RATE);
        let frame = run(&mut analyzer, &noise(BLOCK * 4, 12345)).frame;
        for band in 0..MAX_BANDS {
            assert!(
                frame.bands[band] > 0.2,
                "band {band} read {} on broadband noise: {:?}",
                frame.bands[band],
                frame.bands
            );
        }
        // Broadband noise excites multiple log-spaced bands across the spectrum.
        let lit = frame.bands.iter().filter(|b| **b > 0.3).count();
        assert!(lit >= 6, "only {lit} bands lit by noise: {:?}", frame.bands);
    }

    /// Silence is a measurement, and the frame says so: zero at full
    /// confidence, which is not the same value as no microphone.
    #[test]
    fn silence_reads_zero_at_full_confidence() {
        let mut analyzer = Analyzer::new(RATE);
        let frame = run(&mut analyzer, &silence(BLOCK * 4)).frame;
        assert_eq!(frame.energy, 0.0);
        assert_eq!(frame.onset, 0.0);
        assert_eq!(frame.bands, [0.0; MAX_BANDS]);
        assert_eq!(
            frame.confidence, 1.0,
            "a measured silence is a measurement, not an absence"
        );
    }

    /// The scale, at both ends and in the middle. These numbers are the answer
    /// to "what maps to 1.0", so they are asserted rather than described.
    #[test]
    fn a_full_scale_signal_reads_one_and_sixty_db_down_reads_zero() {
        let mut analyzer = Analyzer::new(RATE);

        // A full-scale sine is −3 dBFS RMS, comfortably past the −6 dBFS top.
        let full = run(&mut analyzer, &tone(1000.0, 1.0, 0, BLOCK * 4)).frame;
        assert_eq!(full.energy, 1.0);
        // The tone is all there is, so its band reads what the broadband level
        // reads — the two scales are the same scale.
        assert_eq!(full.bands[loudest_band(&full)], 1.0);

        // −60 dBFS RMS is the floor. A sine at that RMS has amplitude √2/1000.
        let mut analyzer = Analyzer::new(RATE);
        let floor = run(
            &mut analyzer,
            &tone(1000.0, 0.001 * std::f32::consts::SQRT_2, 0, BLOCK * 4),
        )
        .frame;
        assert!(floor.energy < 0.01, "the floor read {}", floor.energy);

        // And something in between moves through the range rather than sitting
        // at an end: −30 dBFS RMS should land near the middle.
        let mut analyzer = Analyzer::new(RATE);
        let mid = run(
            &mut analyzer,
            &tone(1000.0, 0.0316 * std::f32::consts::SQRT_2, 0, BLOCK * 4),
        )
        .frame;
        assert!(
            (0.4..0.7).contains(&mid.energy),
            "−30 dBFS read {}, which is not usefully in the middle",
            mid.energy
        );
    }

    /// A band measures the part of the signal inside it, so a tone that is all
    /// there is puts its band at the broadband level — and the level tracks the
    /// tone's amplitude rather than being a hit-or-miss indicator.
    #[test]
    fn a_bands_level_is_the_level_of_what_is_in_it() {
        for amplitude in [0.1_f32, 0.03, 0.01] {
            let mut analyzer = Analyzer::new(RATE);
            let frame = run(&mut analyzer, &tone(1000.0, amplitude, 0, BLOCK * 4)).frame;
            let band = frame.bands[loudest_band(&frame)];
            assert!(
                (band - frame.energy).abs() < 0.05,
                "a tone at {amplitude} read {band} in its band and {} broadband",
                frame.energy
            );
        }
    }

    // -- onset --------------------------------------------------------------

    /// The distinction the whole detector exists for: a transient fires and a
    /// sustained tone of the same level does not.
    #[test]
    fn an_onset_fires_on_a_transient_and_not_on_a_sustained_tone() {
        // A click train: four blocks of silence, one block whose first samples
        // are a burst, repeated.
        let mut clicks = Vec::new();
        for i in 0..24 {
            let mut block = silence(BLOCK);
            if i % 4 == 0 {
                for (n, s) in block.iter_mut().take(64).enumerate() {
                    // A short decaying burst of broadband energy.
                    *s = noise(64, 7)[n] * (1.0 - n as f32 / 64.0);
                }
            }
            clicks.extend(block);
        }
        let mut analyzer = Analyzer::new(RATE);
        let mut fired = 0;
        for block in clicks.chunks_exact(BLOCK) {
            if analyzer.analyze(block).onset {
                fired += 1;
            }
        }
        assert!(fired >= 5, "a click train fired {fired} onsets out of 6");

        // The same detector, a sustained tone at a level a click never reaches,
        // for as long: after the note's own attack, nothing.
        let mut analyzer = Analyzer::new(RATE);
        let sustained = tone(440.0, 0.7, 0, BLOCK * 24);
        let mut fired_after_attack = 0;
        for (i, block) in sustained.chunks_exact(BLOCK).enumerate() {
            let onset = analyzer.analyze(block).onset;
            // The first blocks *are* an onset — the note starts. What must not
            // happen is the tone continuing to read as a series of hits.
            if i >= 3 && onset {
                fired_after_attack += 1;
            }
        }
        assert_eq!(
            fired_after_attack, 0,
            "a sustained tone fired {fired_after_attack} onsets after its attack"
        );
    }

    /// The value on the bus is an envelope, not a flag: 1.0 at the hit, decaying
    /// from there, so a frame sampling it at any rate sees something meaningful.
    #[test]
    fn the_onset_signal_is_an_envelope_that_decays_rather_than_an_impulse() {
        let mut analyzer = Analyzer::new(RATE);
        let mut block = silence(BLOCK);
        block[..64].copy_from_slice(&noise(64, 3));
        analyzer.analyze(&silence(BLOCK));
        let hit = analyzer.analyze(&block);
        assert!(hit.onset, "the click did not fire");
        assert_eq!(hit.frame.onset, 1.0);

        // Two half-lives' worth of blocks: down to about a quarter, and every
        // step of the way down is a step down.
        let blocks = (2.0 * ONSET_HALF_LIFE_SECONDS / analyzer.hop_seconds()).ceil() as u32;
        let mut previous = 1.0;
        for _ in 0..blocks {
            let value = analyzer.analyze(&silence(BLOCK)).frame.onset;
            assert!(value < previous, "the envelope did not decay: {value}");
            assert!(value > 0.0, "the envelope collapsed to an impulse");
            previous = value;
        }
        assert!(
            previous < 0.35,
            "after two half-lives the envelope is still at {previous}"
        );
        // ...and it is gone well inside a beat rather than lingering across one.
        for _ in 0..blocks * 2 {
            previous = analyzer.analyze(&silence(BLOCK)).frame.onset;
        }
        assert!(previous < 0.05, "the envelope is still at {previous}");
    }

    /// Verifies adaptive onset thresholding against continuous broadband noise.
    #[test]
    fn a_continuously_busy_spectrum_is_not_a_hit_on_every_block() {
        let mut analyzer = Analyzer::new(RATE);
        // Two seconds for the running mean to learn what usual is here.
        let settle = noise(RATE as usize * 2, 4242);
        for block in settle.chunks_exact(BLOCK) {
            analyzer.analyze(block);
        }

        let busy = noise(RATE as usize * 2, 1717);
        let mut blocks = 0;
        let mut fired = 0;
        for block in busy.chunks_exact(BLOCK) {
            blocks += 1;
            if analyzer.analyze(block).onset {
                fired += 1;
            }
        }
        assert!(
            fired * 10 < blocks,
            "{fired} of {blocks} blocks of steady noise read as onsets"
        );
    }

    /// Silence has no onsets. The absolute floor exists for exactly this: a
    /// running mean of zero makes any numerical wobble infinitely unusual.
    #[test]
    fn silence_never_fires_an_onset() {
        let mut analyzer = Analyzer::new(RATE);
        for _ in 0..40 {
            assert!(!analyzer.analyze(&silence(BLOCK)).onset);
        }
    }

    // -- determinism --------------------------------------------------------

    /// The same blocks give the same frames. Replay depends on the record
    /// rather than on this, but a detector whose output depended on anything
    /// else would be untestable as well as unrecordable.
    #[test]
    fn the_same_blocks_produce_the_same_frames() {
        let signal = noise(BLOCK * 8, 99);
        let analyse = || {
            let mut analyzer = Analyzer::new(RATE);
            signal
                .chunks_exact(BLOCK)
                .map(|b| {
                    let a = analyzer.analyze(b);
                    (a.frame, a.novelty, a.onset)
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(analyse(), analyse());
    }

    /// What one hop of analysis costs, on a host clock, in the audio callback
    /// where it runs. Ignored because it is a measurement rather than an
    /// assertion — `cargo test -p karakuri-audio --release -- --ignored
    /// --nocapture` prints it. The number in the module doc comes from here.
    #[test]
    #[ignore = "a measurement, not an assertion"]
    fn what_one_hop_of_analysis_costs() {
        let signal = noise(BLOCK * 200, 5);
        let mut analyzer = Analyzer::new(RATE);
        let start = std::time::Instant::now();
        let mut sink = 0.0f32;
        for block in signal.chunks_exact(BLOCK) {
            sink += analyzer.analyze(block).novelty;
        }
        let each = start.elapsed().as_secs_f64() / 200.0;
        eprintln!(
            "one {BLOCK}-sample analysis: {:.1} µs (hop is {:.1} ms of audio) [{sink}]",
            each * 1e6,
            analyzer.hop_seconds() * 1000.0
        );
    }

    /// Verifies band edges form a valid, non-empty, and contiguous partition across
    /// hardware sample rates from 8 kHz to 192 kHz.
    #[test]
    fn the_band_edges_are_ordered_non_empty_and_contiguous() {
        for rate in [
            8_000, 11_025, 16_000, 22_050, 32_000, 44_100, 48_000, 88_200, 96_000, 192_000,
        ] {
            let bands = band_bins(rate as f32);
            assert!(
                bands
                    .iter()
                    .all(|(low, high)| *high <= BLOCK / 2 + 1 && *low >= 1),
                "band edges left the spectrum at {rate} Hz: {bands:?}"
            );
            for (i, (low, high)) in bands.iter().enumerate() {
                assert!(low < high, "band {i} at {rate} Hz is empty: {low}..{high}");
                if i > 0 {
                    assert!(
                        bands[i - 1].1 <= *low,
                        "band {i} at {rate} Hz overlaps its neighbour"
                    );
                }
            }
        }
    }
}
