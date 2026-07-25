//! The synthesized signal bus: V1's only kind of signal, since audio and
//! external sync are out of scope. Every value comes from the local
//! oscillator plus an explicit seed — nothing else.
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

/// `energy`, `band*`, and `noise*` have no provider behind them in V1 (audio
/// input is out of scope); they are pure invention shaped to be useful
/// defaults, not measurements. Low but nonzero: a consumer that blends on
/// confidence should still be able to use them as a gentle ambient driver
/// when nothing better is bound, without mistaking them for a real reading.
const CONFIDENCE_INVENTED: f32 = 0.1;

/// A signal bus synthesized entirely from a local [`Oscillator`] and a seed.
///
/// V1 has no external input, so this is the only kind of `SignalBus`: there
/// is nothing to fall back to and nothing to blend against. Holding the
/// oscillator by reference rather than copying its state keeps the bus and
/// the oscillator from being able to disagree — there is exactly one phase in
/// the system, and this just reads it.
pub struct SynthesizedBus<'a> {
    oscillator: &'a Oscillator,
    seed: u64,
}

impl<'a> SynthesizedBus<'a> {
    /// `seed` is the explicit randomness source for every noise signal this
    /// bus produces. Two buses built with the same seed over oscillators fed
    /// the same step sequence agree on every sample, bit for bit — there is
    /// no clock, no thread-derived state, and no interior mutability that a
    /// different call order could perturb.
    pub fn new(oscillator: &'a Oscillator, seed: u64) -> SynthesizedBus<'a> {
        SynthesizedBus { oscillator, seed }
    }
}

impl SignalBus for SynthesizedBus<'_> {
    fn sample(&self, name: &str) -> Sample {
        match name {
            "bpm" => oscillator_derived(self.oscillator.bpm()),
            "beat" => oscillator_derived(pulse(self.oscillator.beat_phase())),
            "bar" => oscillator_derived(pulse(self.oscillator.bar_phase())),
            "energy" => low_confidence(energy(self.oscillator.t())),
            _ if name == "noise" || name.starts_with("noise:") => {
                let stream = noise_stream(name);
                low_confidence(noise(self.seed, stream, self.oscillator.step_index()))
            }
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
fn band_index(name: &str) -> Option<u32> {
    let rest = name.strip_prefix("band")?;
    if rest.is_empty() {
        return Some(0);
    }
    rest.parse().ok()
}

/// The stream salt for a noise signal name: `"noise"` is stream zero: the
/// default. `"noise:<key>"` is an independent, decorrelated stream keyed by
/// `<key>` — anything, not just a number — so several parameters can each
/// bind their own noise without one leaking into another. Binding two params
/// to plain `"noise"` intentionally correlates them; binding one to `"noise"`
/// and the other to `"noise:2"` does not.
fn noise_stream(name: &str) -> u64 {
    match name.strip_prefix("noise:") {
        Some(key) => fnv1a(key.as_bytes()),
        None => 0,
    }
}

/// A noise sample in `0..1`: a pure function of the seed, the stream salt,
/// and the oscillator's step index. Keyed off `step_index` rather than `t` —
/// see the [`Oscillator`] docs — because this is exactly the signal irregular
/// spawning binds to `spawn_rate` (see "Spawn timing" in `docs/ir-spec.md`):
/// its whole job is to depend on which tick history occurred, not merely on
/// how much simulation time elapsed.
fn noise(seed: u64, stream: u64, step_index: u64) -> f32 {
    hash_unit(seed, stream, step_index)
}

/// SplitMix64's finalizer: a fixed, portable bit mix with no external state.
/// Same input bits always produce the same output bits on every platform,
/// which is the whole requirement for "a pure function of its seed and the
/// oscillator phase" — there is no RNG to seed from the OS, the clock, or a
/// thread ID.
fn mix64(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d049bb133111eb);
    x ^= x >> 31;
    x
}

const GOLDEN_GAMMA: u64 = 0x9E3779B97F4A7C15;

/// Combines three integers into one mixed 64-bit value, order-sensitive
/// enough that `(seed, stream, phase)` and any permutation of it land in
/// different places.
fn hash3(a: u64, b: u64, c: u64) -> u64 {
    let mut h = mix64(a);
    h = mix64(h ^ mix64(b).wrapping_add(GOLDEN_GAMMA));
    h = mix64(h ^ mix64(c).wrapping_add(GOLDEN_GAMMA));
    h
}

/// `hash3`, mapped onto `[0, 1)` using the top 24 bits — plenty for a signal
/// that ultimately lands in an `f32` parameter range.
fn hash_unit(a: u64, b: u64, c: u64) -> f32 {
    let h = hash3(a, b, c);
    ((h >> 40) as f32) / (1u64 << 24) as f32
}

/// FNV-1a. Only used to turn a `noise:<key>` suffix into a stream salt; not a
/// cryptographic hash, just a cheap, deterministic way to decorrelate names.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
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

    #[test]
    fn same_seed_and_steps_give_bit_identical_samples() {
        let steps = [(1u8, 1.0 / 60.0), (2, 1.0 / 60.0), (1, 1.0 / 30.0)];
        let osc_a = advance_both(&steps, 128.0);
        let osc_b = advance_both(&steps, 128.0);

        let bus_a = SynthesizedBus::new(&osc_a, 42);
        let bus_b = SynthesizedBus::new(&osc_b, 42);

        for name in [
            "bpm",
            "beat",
            "bar",
            "energy",
            "band",
            "band3",
            "noise",
            "noise:spawn_rate",
        ] {
            assert_eq!(
                bus_a.sample(name),
                bus_b.sample(name),
                "signal {name} diverged between two identically driven oscillators"
            );
        }
    }

    #[test]
    fn different_seeds_diverge_on_noise() {
        let steps = [(1u8, 1.0 / 60.0), (3, 1.0 / 60.0)];
        let osc = advance_both(&steps, 128.0);

        let bus_a = SynthesizedBus::new(&osc, 1);
        let bus_b = SynthesizedBus::new(&osc, 2);

        assert_ne!(bus_a.sample("noise").value, bus_b.sample("noise").value);
    }

    #[test]
    fn distinct_noise_streams_decorrelate() {
        let osc = advance_both(&[(1, 1.0 / 60.0)], 128.0);
        let bus = SynthesizedBus::new(&osc, 7);

        let default_stream = bus.sample("noise").value;
        let spawn_stream = bus.sample("noise:spawn_rate").value;
        let other_stream = bus.sample("noise:turbulence").value;

        assert_ne!(default_stream, spawn_stream);
        assert_ne!(spawn_stream, other_stream);
    }

    #[test]
    fn unknown_signal_name_is_complete_not_missing() {
        let osc = Oscillator::new(120.0);
        let bus = SynthesizedBus::new(&osc, 0);

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
        let bus = SynthesizedBus::new(&osc, 0);

        for name in ["bpm", "beat", "bar"] {
            assert_eq!(bus.sample(name).confidence, 1.0, "{name} should be certain");
        }
    }

    #[test]
    fn invented_signals_are_low_confidence() {
        let osc = Oscillator::new(120.0);
        let bus = SynthesizedBus::new(&osc, 0);

        for name in ["energy", "band", "band2", "noise"] {
            assert!(
                bus.sample(name).confidence < 0.5,
                "{name} should read as invented, not measured"
            );
        }
    }

    #[test]
    fn one_step_of_double_dt_and_two_steps_of_dt_diverge_on_noise_but_not_on_beat() {
        // Same total elapsed time, different tick history.
        let merged = advance_both(&[(1, 0.2)], 90.0);
        let split = advance_both(&[(1, 0.1), (1, 0.1)], 90.0);
        assert_eq!(merged.step_index(), 1);
        assert_eq!(split.step_index(), 2);

        let bus_merged = SynthesizedBus::new(&merged, 99);
        let bus_split = SynthesizedBus::new(&split, 99);

        // `t`-derived signals agree: they only see elapsed time.
        assert_eq!(bus_merged.sample("beat"), bus_split.sample("beat"));
        assert_eq!(bus_merged.sample("bpm"), bus_split.sample("bpm"));

        // The noise signal disagrees: it is keyed on the step count, which is
        // exactly the tick history the two runs differ on. This is what makes
        // it a function of "the oscillator phase" in the step sense rather
        // than the elapsed-time sense — see the module docs.
        assert_ne!(
            bus_merged.sample("noise").value,
            bus_split.sample("noise").value
        );
    }
}
