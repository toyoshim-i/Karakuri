//! Signal binding evaluation and parameter mapping.
//!
//! Evaluates mappings between continuous signals (such as oscillator phase, audio features,
//! or procedural noise generators) and shader/simulation parameters.
//!
//! Each binding samples a signal, applies a shaping [`Curve`], maps the normalized value
//! to a target range, and blends against any manual parameter setting according to
//! the signal's confidence value.

use karakuri_ir::Kind;
use karakuri_signal::{
    AudioFrame, MeasuredBus, NoiseConfig, Oscillator, Sample, SignalBus, SignalId, SynthesizedBus,
    VectorSample,
};

/// Default tempo in beats per minute for new sessions.
pub const DEFAULT_BPM: f32 = 120.0;

/// Special signal name designating the procedural noise generator declared by the binding.
pub const NOISE_SIGNAL: &str = "noise";

/// Prefix designating a published set macro control (for example, `"control:twist"`).
pub const CONTROL_PREFIX: &str = "control:";

/// Transfer function applied to normalized signal samples prior to range mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Curve {
    /// Linear identity function.
    Lin,
    /// Quadratic curve emphasizing peak values.
    Pow2,
    /// Square-root curve emphasizing low values.
    Sqrt,
    /// Smoothstep curve providing eased transitions at both bounds.
    Smooth,
}

/// All available transfer curve variants.
pub const CURVES: [Curve; 4] = [Curve::Lin, Curve::Pow2, Curve::Sqrt, Curve::Smooth];

impl Curve {
    /// Parses a curve name, returning `None` if unrecognized.
    pub fn parse(name: &str) -> Option<Curve> {
        CURVES.into_iter().find(|c| c.name() == name)
    }

    pub fn name(self) -> &'static str {
        match self {
            Curve::Lin => "lin",
            Curve::Pow2 => "pow2",
            Curve::Sqrt => "sqrt",
            Curve::Smooth => "smooth",
        }
    }

    /// Applies the transfer curve to `x`, clamping `x` to [0, 1].
    pub fn apply(self, x: f32) -> f32 {
        let x = if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) };
        match self {
            Curve::Lin => x,
            Curve::Pow2 => x * x,
            Curve::Sqrt => x.sqrt(),
            Curve::Smooth => x * x * (3.0 - 2.0 * x),
        }
    }
}

/// Session signal generator and measurement container.
#[derive(Clone, Copy)]
pub struct Signals {
    oscillator: Oscillator,
    seed: u64,
    audio: Option<AudioFrame>,
}

impl Default for Signals {
    fn default() -> Signals {
        Signals::new(DEFAULT_BPM, 0)
    }
}

impl Signals {
    pub fn new(bpm: f32, seed: u64) -> Signals {
        Signals {
            oscillator: Oscillator::new(bpm),
            seed,
            audio: None,
        }
    }

    /// Install this frame's measured signals, or `None` for "nothing is
    /// measuring".
    ///
    /// **Once per frame, before the frame is rendered**, from the same place
    /// `steps` comes from — a measurement live, a record on replay. A frame's
    /// worth of measurement is latched here and does not change while the frame
    /// is drawn, for the same reason `steps` does not: two bindings sampling
    /// `energy` in one frame have to get one answer, or the record that says
    /// what this frame saw is a record of neither.
    pub fn set_audio(&mut self, audio: Option<AudioFrame>) {
        self.audio = audio;
    }

    /// What [`Signals::set_audio`] last installed.
    pub fn audio(&self) -> Option<&AudioFrame> {
        self.audio.as_ref()
    }

    /// Advance the session clock. `steps` comes from a `tick` record and `dt`
    /// is the fixed simulation step — the same two quantities every Live slot
    /// is advanced by, so the phase a binding reads is the phase at the
    /// instant of the frame's last substep.
    pub fn advance(&mut self, steps: u8, dt: f32) {
        self.oscillator.advance(steps, dt);
    }

    /// **The same signals, with the oscillator read `seconds` earlier.** For a
    /// caller whose clock is behind the session's; `0.0` returns `self`
    /// unchanged, bit for bit. See [`Oscillator::behind`].
    /// Returns signals with the local oscillator evaluated `seconds` in the past.
    pub fn behind(self, seconds: f64) -> Signals {
        Signals {
            oscillator: self.oscillator.behind(seconds),
            ..self
        }
    }

    /// Updates oscillator tempo and applies a phase correction in beats.
    pub fn correct(&mut self, bpm: f32, shift_beats: f32) {
        self.oscillator.correct(bpm, shift_beats);
    }

    pub fn oscillator(&self) -> &Oscillator {
        &self.oscillator
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Samples a signal by pre-resolved [`SignalId`].
    pub fn sample_id(&self, id: SignalId) -> Sample {
        MeasuredBus::new(self.audio.as_ref(), SynthesizedBus::new(&self.oscillator)).sample_id(id)
    }

    /// Samples a vectorized signal by its pre-resolved ID.
    pub fn sample_vector(&self, id: SignalId) -> VectorSample {
        MeasuredBus::new(self.audio.as_ref(), SynthesizedBus::new(&self.oscillator))
            .sample_vector(id)
    }

    /// Samples a signal by name across measured and synthesized sources.
    pub fn sample(&self, name: &str) -> Sample {
        self.sample_id(SignalId::resolve(name))
    }

    /// Samples a procedural noise generator mapped to the unit range [0, 1].
    pub fn noise(&self, config: &NoiseConfig) -> Sample {
        Sample::certain(config.sample(self.seed, &self.oscillator) * 0.5 + 0.5)
    }
}

/// Configuration for binding a signal source to a procedure parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    /// Target layer kind whose parameter is being driven.
    pub layer: Kind,
    /// Specific node index within the layer, or `None` to target all nodes.
    pub index: Option<u32>,
    pub key: String,
    pub signal: String,
    /// Pre-resolved signal identifier for fast lookup.
    pub signal_id: SignalId,
    pub curve: Curve,
    pub range: [f32; 2],
    /// Configuration for generator-driven noise bindings.
    pub noise: Option<NoiseConfig>,
    /// Cached value from the most recent evaluation.
    value: f32,
}

/// Manual parameter override targeted at a specific node or layer.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamWrite {
    /// Target node address `(layer, index)`, or `None` to target all nodes declaring `key`.
    pub at: Option<(Kind, u32)>,
    pub key: String,
    pub value: f32,
}

impl ParamWrite {
    /// Targets all nodes declaring `key`.
    pub fn everywhere(key: impl Into<String>, value: f32) -> ParamWrite {
        ParamWrite {
            at: None,
            key: key.into(),
            value,
        }
    }

    /// Targets a specific node within a layer.
    pub fn at(layer: Kind, index: u32, key: impl Into<String>, value: f32) -> ParamWrite {
        ParamWrite {
            at: Some((layer, index)),
            key: key.into(),
            value,
        }
    }
}

impl Binding {
    pub fn new(
        layer: Kind,
        key: impl Into<String>,
        signal: impl Into<String>,
        curve: Curve,
        range: [f32; 2],
    ) -> Binding {
        let signal = signal.into();
        let signal_id = SignalId::resolve(&signal);
        Binding {
            layer,
            index: None,
            key: key.into(),
            signal,
            signal_id,
            curve,
            range,
            noise: None,
            value: range[0],
        }
    }

    pub fn with_noise(mut self, noise: NoiseConfig) -> Binding {
        self.noise = Some(noise);
        self
    }

    /// Restricts this binding to a specific node index.
    pub fn at(mut self, index: u32) -> Binding {
        self.index = Some(index);
        self
    }

    /// Returns whether this binding targets the node at `index`.
    pub fn covers(&self, index: usize) -> bool {
        self.index.is_none_or(|i| i as usize == index)
    }

    /// Returns the resolved value from the most recent evaluation.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Evaluates the binding against `signals` and blends with `manual`.
    pub fn resolve(&mut self, signals: &Signals, manual: f32) -> f32 {
        let sample = self.sample(signals);
        self.value = blend(manual, self.map(sample.value), sample.confidence);
        self.value
    }

    /// Sets the value directly from a normalized published control position.
    pub fn drive(&mut self, position: f32) -> f32 {
        self.value = self.map(position);
        self.value
    }

    /// Holds the parameter at its manual value without modulation.
    pub fn hold(&mut self, manual: f32) -> f32 {
        self.value = manual;
        self.value
    }

    /// Maps a normalized input value through the curve and target range.
    pub fn map(&self, x: f32) -> f32 {
        let (low, high) = (self.range[0], self.range[1]);
        low + (high - low) * self.curve.apply(x)
    }

    fn sample(&self, signals: &Signals) -> Sample {
        if self.signal == NOISE_SIGNAL {
            signals.noise(&self.noise.unwrap_or_default())
        } else {
            signals.sample_id(self.signal_id)
        }
    }
}

/// Linearly blends between `manual` and `mapped` according to `confidence` clamped to [0, 1].
pub fn blend(manual: f32, mapped: f32, confidence: f32) -> f32 {
    let c = if confidence.is_nan() {
        0.0
    } else {
        confidence.clamp(0.0, 1.0)
    };
    (1.0 - c) * manual + c * mapped
}

#[cfg(test)]
mod tests {
    use super::*;
    use karakuri_signal::NoiseKind;

    const DT: f32 = 1.0 / 60.0;

    /// A session driven by a tick sequence, the way `Deck` drives one.
    fn advanced(bpm: f32, seed: u64, frames: u32) -> Signals {
        let mut signals = Signals::new(bpm, seed);
        for _ in 0..frames {
            signals.advance(1, DT);
        }
        signals
    }

    fn binding(signal: &str, curve: Curve, range: [f32; 2]) -> Binding {
        Binding::new(Kind::L1, "turbulence", signal, curve, range)
    }

    // -- the curve vocabulary ----------------------------------------------

    /// Four names have to be four behaviours, or one of them is decoration.
    #[test]
    fn every_curve_is_distinguishable_from_every_other_at_the_same_input() {
        for x in [0.15_f32, 0.3, 0.7, 0.85] {
            let values: Vec<f32> = CURVES.iter().map(|c| c.apply(x)).collect();
            for i in 0..values.len() {
                for j in (i + 1)..values.len() {
                    assert_ne!(
                        values[i], values[j],
                        "{:?} and {:?} coincide at x = {x}",
                        CURVES[i], CURVES[j]
                    );
                }
            }
        }
    }

    /// The claim each curve is named for: `pow2` sits below the diagonal (the
    /// floor is flattened, the peak is what moves), `sqrt` above it (the floor
    /// is what moves), `smooth` crosses at the middle and is flat at both ends.
    #[test]
    fn each_curve_bends_the_way_its_name_says() {
        let low = 0.25_f32;
        let high = 0.75_f32;

        assert!(Curve::Pow2.apply(low) < low, "pow2 must flatten the floor");
        assert!(
            Curve::Pow2.apply(high) < high,
            "pow2 must emphasise the peak"
        );
        assert!(Curve::Sqrt.apply(low) > low, "sqrt must lift the floor");
        assert!(
            Curve::Sqrt.apply(high) > high,
            "sqrt must compress the peak"
        );
        // Smoothstep is the S: below the diagonal in the bottom half, above it
        // in the top half, and equal at the middle.
        assert!(Curve::Smooth.apply(low) < low);
        assert!(Curve::Smooth.apply(high) > high);
        assert_eq!(Curve::Smooth.apply(0.5), 0.5);

        // Eased at both ends: the first tenth of the input moves the output
        // less than the middle tenth does.
        let ends = Curve::Smooth.apply(0.1) - Curve::Smooth.apply(0.0);
        let middle = Curve::Smooth.apply(0.55) - Curve::Smooth.apply(0.45);
        assert!(ends < middle, "smooth is not eased at the ends");
    }

    #[test]
    fn every_curve_spans_the_whole_range_and_nothing_outside_it() {
        for curve in CURVES {
            assert_eq!(curve.apply(0.0), 0.0, "{curve:?} at 0");
            assert_eq!(curve.apply(1.0), 1.0, "{curve:?} at 1");
            // Out-of-range input is clamped, not extrapolated: `range` is what
            // keeps a bound value inside what the artifact declared.
            assert_eq!(curve.apply(-3.0), 0.0, "{curve:?} below 0");
            assert_eq!(curve.apply(9.0), 1.0, "{curve:?} above 1");
            assert_eq!(curve.apply(f32::NAN), 0.0, "{curve:?} on NaN");
        }
    }

    #[test]
    fn curve_names_round_trip_and_an_unknown_one_is_refused() {
        for curve in CURVES {
            assert_eq!(Curve::parse(curve.name()), Some(curve));
        }
        assert_eq!(Curve::parse("expo"), None);
        assert_eq!(Curve::parse("Lin"), None);
    }

    // -- confidence ---------------------------------------------------------

    #[test]
    fn an_invented_signal_moves_a_param_a_tenth_as_far_as_a_certain_one() {
        let signals = advanced(128.0, 5, 40);
        let sample = signals.sample("energy");
        assert_eq!(sample.confidence, 0.1, "energy is supposed to be invented");

        let manual = 1.0;
        let mut b = binding("energy", Curve::Lin, [0.0, 10.0]);
        let got = b.resolve(&signals, manual);

        // The same sample value, from a provider that is tracking.
        let certain = blend(manual, b.map(sample.value), 1.0);
        assert!(
            (certain - manual).abs() > 1.0,
            "the reference move is too small to measure a tenth of"
        );
        let ratio = (got - manual) / (certain - manual);
        assert!(
            (ratio - 0.1).abs() < 1e-5,
            "an invented signal moved {ratio} of a certain one's distance, not 0.1"
        );
    }

    #[test]
    fn no_signal_name_means_one_thing_to_the_bus_and_another_to_a_binding() {
        let signals = advanced(120.0, 42, 37);
        let manual = 0.5;

        for name in ["bpm", "beat", "bar", "energy", "band", "band3"] {
            let bus = signals.sample(name);
            assert!(bus.confidence > 0.0, "`{name}` is supposed to be provided");
            let mut b = binding(name, Curve::Lin, [0.0, 1.0]);
            assert_eq!(
                b.resolve(&signals, manual),
                blend(manual, b.map(bus.value), bus.confidence),
                "`{name}` means one thing on the bus and another to a binding"
            );
        }

        let bus = signals.sample(NOISE_SIGNAL);
        assert_eq!(
            bus.confidence, 0.0,
            "the bus provides `{NOISE_SIGNAL}`, which is the binding's own name"
        );
        assert_eq!(bus.value, 0.0);
        let mut b = binding(NOISE_SIGNAL, Curve::Lin, [0.0, 1.0]);
        assert_eq!(
            b.resolve(&signals, manual),
            signals.noise(&NoiseConfig::default()).value,
            "a noise binding did not read the generator it declares"
        );
    }

    #[test]
    fn a_signal_nobody_has_ever_heard_of_leaves_the_param_at_its_manual_value() {
        let signals = advanced(128.0, 5, 40);
        let manual = 2.6;
        for name in ["mic_level", "banding", "noise:spawn_rate", ""] {
            let mut b = binding(name, Curve::Lin, [100.0, 200.0]);
            assert_eq!(
                b.resolve(&signals, manual),
                manual,
                "`{name}` moved a param it has no provider for"
            );
        }
    }

    #[test]
    fn confidence_one_writes_the_mapped_value_and_zero_writes_the_manual_one() {
        assert_eq!(blend(2.6, 9.1, 1.0), 9.1);
        assert_eq!(blend(2.6, 9.1, 0.0), 2.6);
        assert_eq!(blend(2.0, 4.0, 0.5), 3.0);
        assert_eq!(blend(2.6, 9.1, 5.0), 9.1);
        assert_eq!(blend(2.6, 9.1, -5.0), 2.6);
        assert_eq!(blend(2.6, 9.1, f32::NAN), 2.6);
    }

    #[test]
    fn a_manual_value_is_the_base_of_the_blend_not_a_competitor_for_the_write() {
        let signals = advanced(120.0, 3, 17);
        let mut certain = binding("beat", Curve::Lin, [0.0, 4.0]);
        assert_eq!(
            certain.resolve(&signals, 0.5),
            certain.resolve(&signals, 400.0)
        );

        let mut invented = binding("energy", Curve::Lin, [0.0, 4.0]);
        let a = invented.resolve(&signals, 0.5);
        let b = invented.resolve(&signals, 1.5);
        assert!(
            (b - a - 0.9).abs() < 1e-5,
            "a 1.0 move in a param at confidence 0.1 should move the write by 0.9, moved {}",
            b - a
        );
    }

    #[test]
    fn a_beat_binding_tracks_the_oscillators_phase() {
        let bpm = 120.0;
        let frames_per_beat = 30;
        let mut signals = Signals::new(bpm, 1);
        let mut b = binding("beat", Curve::Lin, [0.0, 8.0]);

        let mut values = Vec::new();
        for _ in 0..(frames_per_beat * 3) {
            signals.advance(1, DT);
            let manual = 3.0;
            let got = b.resolve(&signals, manual);
            assert_eq!(got, b.map(signals.sample("beat").value));
            values.push(got);
        }

        let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(max - min > 4.0, "a beat binding barely moved: {min}..{max}");

        for i in 0..frames_per_beat {
            let a = values[i];
            let b = values[i + frames_per_beat];
            assert!(
                (a - b).abs() < 1e-3,
                "frame {i} and one beat later differ: {a} vs {b}"
            );
        }

        let beat = &values[frames_per_beat..frames_per_beat * 2];
        let (peak, peak_value) = beat
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .expect("a beat's worth of frames");
        assert_eq!(
            peak,
            frames_per_beat - 1,
            "the peak of a beat is not on the beat"
        );
        assert!(
            (peak_value - 8.0).abs() < 1e-4,
            "the beat instant should reach the top of the range, reached {peak_value}"
        );
    }

    #[test]
    fn the_same_ticks_and_seed_reproduce_every_bound_value_bit_for_bit() {
        let ticks = [1u8, 2, 1, 4, 1, 1, 3, 2];

        let run = |seed: u64| {
            let mut signals = Signals::new(128.0, seed);
            let mut bindings = vec![
                binding("beat", Curve::Pow2, [0.1, 2.4]),
                binding("bar", Curve::Smooth, [0.0, 1.0]),
                binding("energy", Curve::Sqrt, [0.0, 10.0]),
                binding("band3", Curve::Lin, [-1.0, 1.0]),
                binding("nothing_provides_this", Curve::Lin, [0.0, 1.0]),
                binding(NOISE_SIGNAL, Curve::Lin, [4000.0, 16000.0]).with_noise(NoiseConfig {
                    kind: NoiseKind::Fbm { octaves: 4 },
                    rate: 0.5,
                    stream: 3,
                }),
                binding(NOISE_SIGNAL, Curve::Lin, [4000.0, 16000.0]),
            ];
            let mut out = Vec::new();
            for steps in ticks {
                signals.advance(steps, DT);
                for b in &mut bindings {
                    out.push(b.resolve(&signals, 1.0));
                }
            }
            out
        };

        let a = run(19_274);
        let b = run(19_274);
        assert_eq!(a, b, "two identical runs disagreed");

        let other = run(19_275);
        assert_ne!(a, other, "changing the seed changed nothing");
    }

    #[test]
    fn a_noise_binding_reaches_kind_rate_and_stream() {
        let signals = advanced(120.0, 42, 37);
        let range = [4000.0, 16000.0];

        let mut default = binding(NOISE_SIGNAL, Curve::Lin, range);
        let v = default.resolve(&signals, 0.0);
        assert!(
            (range[0]..=range[1]).contains(&v),
            "a noise binding landed outside its range: {v}"
        );

        let sample = |noise: NoiseConfig| {
            binding(NOISE_SIGNAL, Curve::Lin, range)
                .with_noise(noise)
                .resolve(&signals, 0.0)
        };
        let base = NoiseConfig::default();
        assert_ne!(
            sample(base),
            sample(NoiseConfig { stream: 3, ..base }),
            "two streams moved together"
        );
        assert_ne!(
            sample(base),
            sample(NoiseConfig { rate: 0.5, ..base }),
            "rate did not reach the generator"
        );
        assert_ne!(
            sample(base),
            sample(NoiseConfig {
                kind: NoiseKind::White,
                ..base
            }),
            "kind did not reach the generator"
        );
    }

    #[test]
    fn a_noise_binding_takes_full_effect_rather_than_a_tenth_of_one() {
        let mut b = binding(NOISE_SIGNAL, Curve::Lin, [0.0, 1.0]);
        let manual = 0.0;
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        let mut signals = Signals::new(120.0, 7);
        for _ in 0..600 {
            signals.advance(1, DT);
            let v = b.resolve(&signals, manual);
            min = min.min(v);
            max = max.max(v);
        }
        assert!(
            min > 0.1 && max - min > 0.3,
            "a noise binding spanned only {min}..{max} of [0, 1], which a \
             tenth-effect binding could have produced"
        );
    }

    #[test]
    fn noise_is_mapped_into_the_unit_range_not_clamped_at_zero() {
        let mut signals = Signals::new(120.0, 11);
        let config = NoiseConfig {
            kind: NoiseKind::White,
            ..NoiseConfig::default()
        };
        let mut below_half = 0;
        let mut at_floor = 0;
        for _ in 0..600 {
            signals.advance(1, DT);
            let v = signals.noise(&config).value;
            assert!((0.0..=1.0).contains(&v), "noise left the unit range: {v}");
            if v < 0.5 {
                below_half += 1;
            }
            if v == 0.0 {
                at_floor += 1;
            }
        }
        assert!(below_half > 0, "the bottom half of the noise is missing");
        assert!(
            at_floor == 0,
            "{at_floor} samples sat exactly at the floor, which is what clamping looks like"
        );
    }

    #[test]
    fn a_bpm_binding_saturates_because_bpm_is_not_a_unit_range_signal() {
        let signals = advanced(128.0, 0, 10);
        let mut b = binding("bpm", Curve::Lin, [0.0, 4.0]);
        assert_eq!(b.resolve(&signals, 1.0), 4.0);
    }

    #[test]
    fn binding_pre_resolves_signal_id() {
        let b_beat = binding("beat", Curve::Lin, [0.0, 1.0]);
        assert_eq!(b_beat.signal_id, SignalId::Beat);

        let b_energy = binding("energy", Curve::Lin, [0.0, 1.0]);
        assert_eq!(b_energy.signal_id, SignalId::Energy);

        let b_band3 = binding("band3", Curve::Lin, [0.0, 1.0]);
        assert_eq!(b_band3.signal_id, SignalId::Band(3));

        let b_custom = binding("custom_signal", Curve::Lin, [0.0, 1.0]);
        assert_eq!(b_custom.signal_id, SignalId::resolve("custom_signal"));

        let signals = advanced(120.0, 0, 15);
        let mut b_test = binding("beat", Curve::Lin, [0.0, 1.0]);
        let val = b_test.resolve(&signals, 0.0);
        let expected_sample = signals.sample_id(SignalId::Beat);
        assert_eq!(val, expected_sample.value);
    }
}
