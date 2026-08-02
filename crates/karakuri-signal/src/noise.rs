//! A parameterised noise generator.
//!
//! Irregular spawning binds a noise signal to `spawn_rate` instead of baking
//! a distribution into the engine, specifically so its "depth and period are
//! declarative and adjustable" (see "Spawn timing" in `docs/ir-spec.md`).
//! Depth is the `bind` record's `range`. Period lives here, as [`NoiseConfig::rate`]:
//! without it, noise runs at a fixed rate no consumer can turn, which is
//! exactly the "fixed property nobody can reach" that section rejects Poisson
//! for.
//!
//! Every kind shares one structure: a lattice coordinate, `beats * rate`,
//! split into an integer index and a fraction. What differs between kinds is
//! only what happens with that split — see [`NoiseKind`].
//!
//! Deterministic: a [`NoiseConfig::sample`] call is a pure function of an
//! explicit `seed`, the config, and the oscillator's `t` and `bpm`. No clock,
//! no interior mutability, nothing thread-derived.

use crate::oscillator::Oscillator;

/// Which lattice-noise algorithm to use. All four take the same lattice
/// coordinate and split it into an integer index and a fraction; they differ
/// only in what they do with that split.
///
/// Names match the vocabulary the IR's noise builtins already use
/// (`value_noise`, `perlin`, `fbm` in `docs/ir-spec.md`) rather than
/// inventing a parallel set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoiseKind {
    /// The hashed value at the lattice index, held constant across the whole
    /// cell — no interpolation. This is what gives white noise a period at
    /// all: instead of running at the fixed step rate, it now holds each
    /// value for `1 / rate` beats before jumping to the next. Discontinuous
    /// at every lattice boundary.
    White,
    /// Hashed lattice values interpolated with a smoothstep. Continuous.
    Value,
    /// One-dimensional gradient noise: a hashed slope at each lattice point,
    /// evaluated against the signed offset from that point and interpolated
    /// with a smoothstep. Continuous, and exactly zero at every integer
    /// lattice coordinate — the interpolation weight and the offset both
    /// vanish there.
    Perlin,
    /// A sum of `octaves` perlin layers at doubling frequency and halving
    /// amplitude, normalised by the total amplitude so the sum stays inside
    /// roughly the same range as a single octave. `octaves` plays the role
    /// the IR's `fbm` builtin gives its compile-time octave count — fixed
    /// per configuration, not something that varies sample to sample — but
    /// it is an ordinary field here: nothing on this side needs to unroll a
    /// shader loop, so there is no reason to force it to a Rust constant.
    Fbm { octaves: u32 },
}

impl Default for NoiseKind {
    /// Perlin. Art tooling assumes Perlin first, and it is the smooth
    /// default — the useful one when a `bind` record does not ask for
    /// anything more specific.
    fn default() -> NoiseKind {
        NoiseKind::Perlin
    }
}

/// A noise generator's full configuration: kind, period, and decorrelation.
///
/// This is what a caller who needs a noise stream — for instance, an engine
/// resolving a `bind` record that names one — constructs and calls
/// [`sample`](NoiseConfig::sample) on directly. It is the **only** way to
/// reach noise: [`crate::SynthesizedBus`] deliberately does not answer a
/// `"noise"` name, because a `&str` is what
/// [`SignalBus::sample`](crate::SignalBus::sample) takes and encoding a
/// four-field configuration into one has no natural, collision-free grammar.
/// A parameterless stand-in on the bus would not fix that; it would only put a
/// second, weaker meaning behind the same name.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseConfig {
    pub kind: NoiseKind,
    /// Cycles per beat. Tempo-relative rather than seconds-relative: the
    /// lattice coordinate is `beats * rate`, where `beats = t * bpm / 60`, so
    /// `rate = 1.0` advances one lattice cell per beat and `rate = 4.0`
    /// advances four. The local oscillator is the single source of truth for
    /// phase, so deriving noise phase from its `t`/`bpm` rather than from
    /// seconds directly keeps there from being a second, independent notion
    /// of "how fast" in the system. See the module docs for the one
    /// consequence of this a non-musical consumer should know about.
    pub rate: f32,
    /// A caller-chosen id, folded into the hash alongside `seed` so two
    /// `NoiseConfig`s that differ only in `stream` decorrelate completely.
    /// This replaces an earlier `"noise:<key>"` name-string convention: that
    /// syntax lived nowhere in the record format and risked colliding with
    /// however signal names end up validated. A plain field the caller sets
    /// is enough, and it is the caller's job to keep two streams that should
    /// stay independent (e.g. one bound to `spawn_rate`, another to
    /// `turbulence`) at different values.
    pub stream: u64,
}

impl Default for NoiseConfig {
    /// Perlin, one cycle per beat, stream `0` — what a `bind` record naming
    /// `noise` and saying nothing else asks for.
    fn default() -> NoiseConfig {
        NoiseConfig {
            kind: NoiseKind::default(),
            rate: 1.0,
            stream: 0,
        }
    }
}

impl NoiseConfig {
    /// Sample this configuration at the oscillator's current phase.
    ///
    /// Deterministic: a pure function of `seed`, `self`, and
    /// `oscillator.t()`/`oscillator.bpm()` — nothing else is read, so the
    /// same seed over an oscillator fed the same sequence of
    /// [`Oscillator::advance`] calls reproduces the same sample, bit for bit.
    pub fn sample(&self, seed: u64, oscillator: &Oscillator) -> f32 {
        // `elapsed_beats`, not `beats`: a noise stream follows a tempo
        // correction, because cycles-per-beat is a rate and the accumulator
        // keeps it continuous, but a **phase** correction does not re-hash it.
        // Realigning the beat grid with a room is no reason for a flicker with
        // no musical intent to jump. See `Oscillator`'s module doc, which is
        // also where `docs/roadmap.md`'s open question about this is answered.
        let lattice = oscillator.elapsed_beats() * self.rate as f64;
        sample_kind(self.kind, seed, self.stream, lattice)
    }
}

fn sample_kind(kind: NoiseKind, seed: u64, stream: u64, lattice: f64) -> f32 {
    match kind {
        NoiseKind::White => white(seed, stream, lattice),
        NoiseKind::Value => value(seed, stream, lattice),
        NoiseKind::Perlin => perlin(seed, stream, lattice),
        NoiseKind::Fbm { octaves } => fbm(seed, stream, lattice, octaves),
    }
}

/// Splits a lattice coordinate into its integer index and the `0..1`
/// fraction past it. The one piece of structure every [`NoiseKind`] shares.
fn lattice_split(x: f64) -> (i64, f32) {
    let index = x.floor();
    (index as i64, (x - index) as f32)
}

fn white(seed: u64, stream: u64, lattice: f64) -> f32 {
    let (index, _fraction) = lattice_split(lattice);
    hash_signed(seed, stream, index)
}

fn value(seed: u64, stream: u64, lattice: f64) -> f32 {
    let (index, fraction) = lattice_split(lattice);
    let a = hash_signed(seed, stream, index);
    let b = hash_signed(seed, stream, index + 1);
    lerp(a, b, smoothstep(fraction))
}

fn perlin(seed: u64, stream: u64, lattice: f64) -> f32 {
    let (index, fraction) = lattice_split(lattice);
    // A hashed "slope" at each lattice point, in [-1, 1] — the 1-D analogue
    // of a gradient vector.
    let slope0 = hash_signed(seed, stream, index);
    let slope1 = hash_signed(seed, stream, index + 1);
    let d0 = slope0 * fraction;
    let d1 = slope1 * (fraction - 1.0);
    lerp(d0, d1, smoothstep(fraction))
}

fn fbm(seed: u64, stream: u64, lattice: f64, octaves: u32) -> f32 {
    let octaves = octaves.max(1);
    let mut sum = 0.0f32;
    let mut total_amplitude = 0.0f32;
    let mut amplitude = 1.0f32;
    let mut frequency = lattice;
    for octave in 0..octaves {
        // Each octave gets its own decorrelated stream, salted off the base
        // stream, so the layers do not just repeat the same shape at a
        // different frequency.
        let octave_stream = stream ^ mix64(GOLDEN_GAMMA.wrapping_mul(octave as u64 + 1));
        sum += perlin(seed, octave_stream, frequency) * amplitude;
        total_amplitude += amplitude;
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    sum / total_amplitude
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A hashed value at an integer lattice index, mapped to `[-1, 1)`.
fn hash_signed(seed: u64, stream: u64, index: i64) -> f32 {
    2.0 * hash_unit(seed, stream, index as u64) - 1.0
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
/// enough that `(seed, stream, index)` and any permutation of it land in
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

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_KINDS: [NoiseKind; 4] = [
        NoiseKind::White,
        NoiseKind::Value,
        NoiseKind::Perlin,
        NoiseKind::Fbm { octaves: 4 },
    ];

    #[test]
    fn each_kind_is_a_pure_function_of_seed_config_and_oscillator_state() {
        let mut osc_a = Oscillator::new(128.0);
        osc_a.advance(1, 1.0 / 60.0);
        osc_a.advance(3, 1.0 / 60.0);

        let mut osc_b = Oscillator::new(128.0);
        osc_b.advance(1, 1.0 / 60.0);
        osc_b.advance(3, 1.0 / 60.0);

        for kind in ALL_KINDS {
            let config = NoiseConfig {
                kind,
                rate: 2.5,
                stream: 9,
            };

            // Repeated sampling of the same state agrees...
            assert_eq!(config.sample(11, &osc_a), config.sample(11, &osc_a));
            // ...and so does a second oscillator fed the identical sequence.
            assert_eq!(
                config.sample(11, &osc_a),
                config.sample(11, &osc_b),
                "{kind:?} diverged between two identically driven oscillators"
            );
        }
    }

    #[test]
    fn kinds_produce_different_values_at_the_same_lattice_position() {
        let seed = 555u64;
        let stream = 0u64;
        let lattice = 2.37;

        let values: Vec<f32> = ALL_KINDS
            .iter()
            .map(|&kind| sample_kind(kind, seed, stream, lattice))
            .collect();

        for i in 0..values.len() {
            for j in (i + 1)..values.len() {
                assert_ne!(
                    values[i], values[j],
                    "{:?} and {:?} coincided at the same lattice position",
                    ALL_KINDS[i], ALL_KINDS[j]
                );
            }
        }
    }

    #[test]
    fn white_is_discontinuous_across_a_lattice_boundary_others_are_not() {
        let eps = 1e-4_f64;
        let boundary = 5.0_f64;
        let seed = 12345u64;
        let continuous_kinds = [
            NoiseKind::Value,
            NoiseKind::Perlin,
            NoiseKind::Fbm { octaves: 4 },
        ];

        let mut max_white_jump = 0.0f32;
        for stream in 0..32u64 {
            let below = sample_kind(NoiseKind::White, seed, stream, boundary - eps);
            let above = sample_kind(NoiseKind::White, seed, stream, boundary + eps);
            max_white_jump = max_white_jump.max((above - below).abs());

            for kind in continuous_kinds {
                let below = sample_kind(kind, seed, stream, boundary - eps);
                let above = sample_kind(kind, seed, stream, boundary + eps);
                let jump = (above - below).abs();
                assert!(
                    jump < 1e-3,
                    "{kind:?} stream {stream} jumped {jump} across a lattice boundary at eps={eps}"
                );
            }
        }

        assert!(
            max_white_jump > 0.1,
            "expected white noise to show a clear jump across a lattice boundary \
             in at least one of 32 streams; max observed was {max_white_jump}"
        );
    }

    #[test]
    fn perlin_is_exactly_zero_at_integer_lattice_points() {
        // A structural check of the same continuity property: the
        // interpolation weight and the gradient offset both vanish exactly
        // at an integer coordinate, regardless of seed or stream.
        for stream in 0..8u64 {
            assert_eq!(sample_kind(NoiseKind::Perlin, 777, stream, 12.0), 0.0);
        }
    }

    #[test]
    fn rate_scales_how_often_the_lattice_cell_changes() {
        let bpm = 120.0;
        let dt = 1.0 / 60.0;
        let seed = 7u64;
        let slow = NoiseConfig {
            kind: NoiseKind::White,
            rate: 1.0,
            stream: 0,
        };
        let fast = NoiseConfig {
            kind: NoiseKind::White,
            rate: 4.0,
            stream: 0,
        };

        let mut osc = Oscillator::new(bpm);
        let mut slow_prev = slow.sample(seed, &osc);
        let mut fast_prev = fast.sample(seed, &osc);
        let mut slow_changes = 0u32;
        let mut fast_changes = 0u32;

        // 600 steps at dt = 1/60s is 10 simulated seconds, well below any
        // step-count-driven behaviour (there is none any more — see
        // Oscillator's docs) and far more than enough lattice crossings at
        // 120 bpm to compare rates reliably.
        for _ in 0..600 {
            osc.advance(1, dt);
            let s = slow.sample(seed, &osc);
            let f = fast.sample(seed, &osc);
            if s != slow_prev {
                slow_changes += 1;
                slow_prev = s;
            }
            if f != fast_prev {
                fast_changes += 1;
                fast_prev = f;
            }
        }

        assert!(
            slow_changes > 0,
            "expected at least one lattice crossing at rate 1 over 10 simulated seconds"
        );
        let ratio = fast_changes as f32 / slow_changes as f32;
        assert!(
            (2.5..6.0).contains(&ratio),
            "expected roughly a 4x crossing-rate increase from a 4x rate increase; \
             slow={slow_changes} fast={fast_changes} ratio={ratio}"
        );
    }

    #[test]
    fn distinct_streams_decorrelate() {
        let mut osc = Oscillator::new(120.0);
        osc.advance(37, 1.0 / 60.0);

        let seed = 3u64;
        let a = NoiseConfig {
            kind: NoiseKind::Perlin,
            rate: 1.0,
            stream: 0,
        }
        .sample(seed, &osc);
        let b = NoiseConfig {
            kind: NoiseKind::Perlin,
            rate: 1.0,
            stream: 1,
        }
        .sample(seed, &osc);

        assert_ne!(a, b);
    }

    /// **Which clock noise runs off**, which is `docs/roadmap.md`'s open
    /// question about a tempo-relative rate under correction, answered in a
    /// test rather than only in prose: a *tempo* correction reaches a noise
    /// stream, and a *phase* correction does not.
    ///
    /// A phase correction realigns the beat grid with a room. Re-hashing every
    /// noise stream because of it would make a flicker with no musical intent
    /// jump whenever the tracker nudged the grid.
    #[test]
    fn noise_follows_a_tempo_correction_and_not_a_phase_one() {
        let config = NoiseConfig::default();
        let seed = 4242;

        let mut osc = Oscillator::new(120.0);
        osc.advance(37, 1.0 / 60.0);
        let before = config.sample(seed, &osc);

        // A phase correction: the beat grid moves, the noise does not.
        osc.correct(osc.bpm(), 0.37);
        assert_eq!(
            config.sample(seed, &osc),
            before,
            "a phase correction moved a noise stream"
        );

        // A tempo correction is continuous at the instant it lands — no jump —
        // and from there the stream runs at the corrected rate.
        osc.correct(180.0, 0.0);
        assert_eq!(
            config.sample(seed, &osc),
            before,
            "a tempo correction made a noise stream jump"
        );
        let mut slow = Oscillator::new(120.0);
        slow.advance(37, 1.0 / 60.0);
        osc.advance(30, 1.0 / 60.0);
        slow.advance(30, 1.0 / 60.0);
        assert_ne!(
            config.sample(seed, &osc),
            config.sample(seed, &slow),
            "a corrected tempo did not change how fast a noise stream runs"
        );
    }

    #[test]
    fn default_config_is_perlin_one_cycle_per_beat_stream_zero() {
        let config = NoiseConfig::default();
        assert_eq!(config.kind, NoiseKind::Perlin);
        assert_eq!(config.rate, 1.0);
        assert_eq!(config.stream, 0);
    }
}
