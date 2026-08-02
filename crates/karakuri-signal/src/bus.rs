//! The synthesized signal bus: V1's only kind of signal, since audio and
//! external sync are out of scope. Every value on it is a pure function of the
//! local oscillator's `t` and `bpm` — nothing else.
//!
//! **Nothing seeded lives here.** A signal a name alone can describe belongs on
//! the bus; a generator with kind, rate, stream and octaves to say does not,
//! because [`SignalBus::sample`](crate::SignalBus::sample) takes a `&str` and
//! there is no collision-free grammar for four fields inside one. Noise is
//! therefore reached through [`NoiseConfig`](crate::NoiseConfig), and the name
//! `"noise"` belongs to the `bind` record that declares one — see
//! [`sample`](SynthesizedBus::sample).
//!
//! [`SynthesizedBus`] never fails to resolve a name and never returns an
//! `Option`; see [`SignalBus`](crate::SignalBus) for why. Consumers branch on
//! [`Sample::confidence`](crate::Sample::confidence) instead.

use crate::oscillator::Oscillator;
use crate::{Sample, SignalBus};

/// Oscillator-derived values (`bpm`, `beat`, `bar`) are the local oscillator's
/// own ground truth — the single source of truth per the project invariants —
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
/// V1 has no external input, so this is the only kind of `SignalBus`: there
/// is nothing to fall back to and nothing to blend against. Holding the
/// oscillator by reference rather than copying its state keeps the bus and
/// the oscillator from being able to disagree — there is exactly one phase in
/// the system, and this just reads it.
///
/// **No seed.** Everything the bus answers is a deterministic waveform in `t`
/// and `bpm`; the explicit seed stream the determinism invariant asks for is
/// [`NoiseConfig::sample`](crate::NoiseConfig::sample)'s, where the randomness
/// actually is. A seed carried here and read by nothing would look like a
/// randomness source that had been accounted for.
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
        match name {
            "bpm" => oscillator_derived(self.oscillator.bpm()),
            "beat" => oscillator_derived(pulse(self.oscillator.beat_phase())),
            "bar" => oscillator_derived(pulse(self.oscillator.bar_phase())),
            "energy" => low_confidence(energy(self.oscillator.t())),
            // **No `"noise"` here, deliberately.** A noise generator has kind,
            // rate, stream and octaves to say, and `sample` takes a name and
            // nothing else — so noise is reached through [`NoiseConfig`], and
            // a `bind` record whose `signal` is `"noise"` names the generator
            // it declares. Answering the same name here too would give it two
            // meanings, at two confidences, resolved by which consumer asked;
            // `docs/ir-spec.md` requires two vocabularies that meet in one
            // decoder to be "disjoint by name", not merely disjoint in
            // practice. The bus stays complete either way: the arm below
            // answers every name it does not know.
            _ => match band_index(name) {
                Some(index) => low_confidence(band(self.oscillator.t(), index)),
                None => Sample::synthesized(0.0),
            },
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

/// Recognizes `"band"` (index 0) and `"band<N>"` for a decimal `N`. Anything
/// else — including near-misses like `"banding"` — is not a band signal and
/// falls through to the bus's unknown-name case, rather than guessing.
///
/// Shared with [`measured`](crate::measured) rather than copied into it: a
/// measured `band3` and an invented one have to be the same name, or a binding
/// would change which signal it means when a microphone appears.
pub(crate) fn band_index(name: &str) -> Option<u32> {
    let rest = name.strip_prefix("band")?;
    if rest.is_empty() {
        return Some(0);
    }
    rest.parse().ok()
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

    /// **`"noise"` is not a bus name.** A noise generator has kind, rate,
    /// stream and octaves to say and `sample` takes only a name, so noise is
    /// `NoiseConfig`'s and the name belongs to the `bind` record that declares
    /// one. Answering it here as well would give one name two meanings at two
    /// confidences — the shape `docs/ir-spec.md` refuses when it asks for
    /// vocabularies that are "disjoint by name" rather than in practice.
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
