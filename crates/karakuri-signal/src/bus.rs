//! The synthesized signal bus: computes deterministic signals derived from the local
//! oscillator's `t` and `bpm`.
//!
//! The bus answers only unseeded signals identifiable by name alone. Parameterized
//! noise generators require explicit configuration and are sampled through
//! [`NoiseConfig`](crate::NoiseConfig).
//!
//! [`SynthesizedBus`] resolves all valid names without returning `Option`; consumers
//! evaluate [`Sample::confidence`](crate::Sample::confidence).

use crate::oscillator::Oscillator;
use crate::{Sample, SignalBus, SignalId};

/// Oscillator-derived values (`bpm`, `beat`, `bar`) are the local oscillator's
/// own ground truth — the single source of truth for phase and tempo —
/// so they carry full confidence.
const CONFIDENCE_OSCILLATOR: f32 = 1.0;

/// `energy` and `band*` have no provider behind them in V1 (audio input is out
/// of scope); they are pure invention shaped to be useful defaults, not
/// measurements. Low but nonzero: a consumer that blends on confidence should
/// still be able to use them as a gentle ambient driver when nothing better is
/// bound, without mistaking them for a real reading.
const CONFIDENCE_INVENTED: f32 = 0.1;

/// A signal bus synthesized entirely from a local [`Oscillator`].
///
/// Holds the oscillator by reference to sample deterministic waveforms as a function
/// of `t` and `bpm`. Randomness is handled separately via [`NoiseConfig`](crate::NoiseConfig).
pub struct SynthesizedBus<'a> {
    oscillator: &'a Oscillator,
}

impl<'a> SynthesizedBus<'a> {
    /// Two buses over oscillators fed the same step sequence agree on every
    /// sample, bit for bit — there is no clock, no thread-derived state, and
    /// no interior mutability that a different call order could perturb.
    pub fn new(oscillator: &'a Oscillator) -> SynthesizedBus<'a> {
        SynthesizedBus { oscillator }
    }
}

impl SignalBus for SynthesizedBus<'_> {
    fn sample(&self, name: &str) -> Sample {
        self.sample_id(SignalId::resolve(name))
    }

    fn sample_id(&self, id: SignalId) -> Sample {
        match id {
            SignalId::Bpm => oscillator_derived(self.oscillator.bpm()),
            SignalId::Beat => oscillator_derived(pulse(self.oscillator.beat_phase())),
            SignalId::Bar => oscillator_derived(pulse(self.oscillator.bar_phase())),
            SignalId::Energy => low_confidence(energy(self.oscillator.t())),
            SignalId::Onset => Sample::synthesized(0.0),
            SignalId::Band(index) => low_confidence(band(self.oscillator.t(), index as u32)),
            SignalId::Custom(_) => Sample::synthesized(0.0),
        }
    }
}

fn low_confidence(value: f32) -> Sample {
    Sample {
        value,
        confidence: CONFIDENCE_INVENTED,
    }
}

fn oscillator_derived(value: f32) -> Sample {
    Sample {
        value,
        confidence: CONFIDENCE_OSCILLATOR,
    }
}

/// A percussive envelope from a `0..1` phase: `1.0` at the instant of the
/// beat (or bar), decaying towards `0.0` as the next one approaches. Used to
/// turn the oscillator's raw phase into a signal shaped like the pulse a beat
/// detector would emit, so binding `beat` to a parameter looks like a hit
/// rather than a sawtooth ramp.
fn pulse(phase: f32) -> f32 {
    (1.0 - phase).powf(3.0)
}

/// A slow, always-on "breathing" envelope in `0..1`, standing in for an audio
/// energy signal V1 has no input for. Three incommensurate sine waves summed
/// together avoid the single-frequency look of one `sin(t)`, while staying a
/// pure function of simulation time — no seed needed, since this is a
/// deterministic waveform rather than randomness.
fn energy(t: f64) -> f32 {
    let raw = (t * 0.9).sin() + 0.5 * (t * 0.37 + 1.3).sin() + 0.25 * (t * 1.71 + 2.7).sin();
    (raw / 1.75) as f32 * 0.5 + 0.5
}

/// A synthesized per-band envelope, same shape as [`energy`] but at a
/// distinct frequency and phase offset per band index so that `band0..bandN`
/// decorrelate from one another, the way a rough spectral view would.
fn band(t: f64, index: u32) -> f32 {
    let frequency = 0.6 + index as f64 * 0.83;
    let phase_offset = index as f64 * 1.9;
    let raw = (t * frequency + phase_offset).sin();
    raw as f32 * 0.5 + 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    fn advance_both(steps: &[(u8, f32)], bpm: f32) -> Oscillator {
        let mut osc = Oscillator::new(bpm);
        for &(s, dt) in steps {
            osc.advance(s, dt);
        }
        osc
    }

    /// Every name the bus answers. One list, so a signal added without a
    /// test here is a signal that does not appear in any of them.
    const PROVIDED: [&str; 6] = ["bpm", "beat", "bar", "energy", "band", "band3"];

    #[test]
    fn the_same_steps_give_bit_identical_samples() {
        let steps = [(1u8, 1.0 / 60.0), (2, 1.0 / 60.0), (1, 1.0 / 30.0)];
        let osc_a = advance_both(&steps, 128.0);
        let osc_b = advance_both(&steps, 128.0);

        let bus_a = SynthesizedBus::new(&osc_a);
        let bus_b = SynthesizedBus::new(&osc_b);

        for name in PROVIDED {
            assert_eq!(
                bus_a.sample(name),
                bus_b.sample(name),
                "signal {name} diverged between two identically driven oscillators"
            );
        }
    }

    /// Verifies that `"noise"` is rejected by the bus and must be sampled via `NoiseConfig`.
    #[test]
    fn noise_is_not_a_name_the_bus_answers() {
        let steps = [(1u8, 1.0 / 60.0), (3, 1.0 / 60.0)];
        let osc = advance_both(&steps, 128.0);
        let bus = SynthesizedBus::new(&osc);

        let s = bus.sample("noise");
        assert_eq!(s.value, 0.0);
        assert_eq!(
            s.confidence, 0.0,
            "the bus claimed to provide `noise`, which is the binding's name"
        );
    }

    #[test]
    fn colon_syntax_is_no_longer_special_cased() {
        // Multi-stream noise now goes through `NoiseConfig` directly (see
        // `noise::tests::distinct_streams_decorrelate`), not through a
        // `"noise:<key>"` name string. A name using that old convention is
        // just an unrecognized name now — still complete, still low
        // confidence, not a guess at what the caller meant.
        let osc = Oscillator::new(120.0);
        let bus = SynthesizedBus::new(&osc);

        let s = bus.sample("noise:spawn_rate");
        assert_eq!(s.value, 0.0);
        assert_eq!(s.confidence, 0.0);
    }

    #[test]
    fn unknown_signal_name_is_complete_not_missing() {
        let osc = Oscillator::new(120.0);
        let bus = SynthesizedBus::new(&osc);

        let s = bus.sample("some_signal_nobody_ever_defined");
        assert_eq!(s.value, 0.0);
        assert_eq!(s.confidence, 0.0);

        // A near-miss on a recognized prefix is still "unknown", not a guess.
        let s = bus.sample("banding");
        assert_eq!(s.confidence, 0.0);
    }

    #[test]
    fn oscillator_derived_signals_are_high_confidence() {
        let osc = Oscillator::new(120.0);
        let bus = SynthesizedBus::new(&osc);

        for name in ["bpm", "beat", "bar"] {
            assert_eq!(bus.sample(name).confidence, 1.0, "{name} should be certain");
        }
    }

    #[test]
    fn invented_signals_are_low_confidence() {
        let osc = Oscillator::new(120.0);
        let bus = SynthesizedBus::new(&osc);

        for name in ["energy", "band", "band2"] {
            assert!(
                bus.sample(name).confidence < 0.5,
                "{name} should read as invented, not measured"
            );
        }
    }

    #[test]
    fn same_total_time_gives_the_same_synthesized_signals_regardless_of_tick_granularity() {
        // Same total elapsed time, different tick history: one call of
        // steps=1/dt=0.2 vs. two calls of steps=1/dt=0.1. Every signal on the
        // bus is a function of `t` and `bpm` alone (see `Oscillator`'s
        // docs), so all of them agree here.
        let merged = advance_both(&[(1, 0.2)], 90.0);
        let split = advance_both(&[(1, 0.1), (1, 0.1)], 90.0);

        let bus_merged = SynthesizedBus::new(&merged);
        let bus_split = SynthesizedBus::new(&split);

        for name in PROVIDED {
            assert_eq!(
                bus_merged.sample(name),
                bus_split.sample(name),
                "signal {name} should not depend on tick call granularity, only on total elapsed time"
            );
        }
    }
}
