//! Signal bindings: what turns a `bind` record into a uniform write.
//!
//! A binding maps one signal onto one `param` of one procedure, every frame.
//! Five steps, and the fourth is the one that carries the design:
//!
//! 1. sample the signal **by name**, from a bus that is always complete;
//! 2. put the sample's value through the binding's [`Curve`];
//! 3. map that onto the binding's `range`;
//! 4. **blend against the param's own value by the sample's confidence**;
//! 5. write the result as a uniform, on the path a `--param` override takes.
//!
//! ## Confidence, and why the demo is quiet
//!
//! `Sample`'s own documentation asks for step 4: confidence "is never a flag:
//! a consumer blends on it rather than testing it". The consequence is worth
//! following rather than softening. `beat`, `bar` and `bpm` come off the local
//! oscillator, which the project invariants call the single source of truth,
//! so they carry confidence 1.0 and a binding to them takes full effect today.
//! `energy` and the bands are invented when nothing is measuring, so they carry
//! 0.1 and move a parameter by a tenth of what the same number from a real
//! provider would. That is the system being honest about what it knows. A demo
//! made livelier by ignoring confidence would be a lie that has to be
//! unwritten.
//!
//! Audio has landed, and this is what "with nothing else changed" turned out to
//! mean: [`Signals::set_audio`] puts one frame's measurements on the session
//! once per frame, `sample` layers them over the synthesized bus, and every
//! step below — curve, range, blend, write — is untouched. The same `energy`
//! binding that moved a tenth of the way now moves all of it, because the same
//! name came back with confidence 1.0 instead of 0.1.
//!
//! Nothing here asks whether a provider exists. [`SignalBus::sample`] cannot
//! fail and does not return an `Option`; a name nobody has ever heard of comes
//! back with confidence 0.0, and step 4 then writes the param's own value
//! unchanged. The absence of a provider is not a branch, it is a coefficient.
//!
//! ## Range, and why the input is clamped
//!
//! Steps 2 and 3 assume the sample is in `[0, 1]`, which is what makes `range`
//! mean what it says. Two signals are not:
//!
//! - **`bpm`** is a tempo in beats per minute. It clamps to 1.0 and a binding
//!   to it is therefore pinned at the top of its range. Bind `beat` or `bar`
//!   instead; this is recorded in `docs/ir-spec.md` rather than papered over
//!   with an invented normalisation range.
//! - **noise** is signed, in `[-1, 1)`. A binding maps it to `[0, 1]` before
//!   the curve ([`Signals::noise`]), because clamping would throw away the
//!   half of the signal below zero — a `spawn_rate` bound to noise would sit
//!   at the bottom of its range half the time.
//!
//! Anything else out of range is clamped rather than extrapolated: `range` is
//! also what keeps a bound value inside what the artifact declared.
//!
//! ## One oscillator, one session
//!
//! [`Signals`] is that oscillator plus the seed every noise stream comes from,
//! and there is one per session — `Deck` owns it, advances it once per frame
//! by the same `steps` every Live slot advances by, and hands it to each Set.
//! Two oscillators would be two truths about phase and tempo. A binding is
//! therefore a pure function of the tick sequence and the seed, which is what
//! puts it inside the determinism invariant rather than beside it.

use karakuri_ir::Kind;
use karakuri_signal::{
    AudioFrame, MeasuredBus, NoiseConfig, Oscillator, Sample, SignalBus, SynthesizedBus,
};

/// The tempo a session runs at until something corrects it. There is no tempo
/// record in the v0.2 vocabulary and no external sync yet, so this is a
/// starting value rather than a measurement — the same status `BEATS_PER_BAR`
/// has in `karakuri-signal`.
pub const DEFAULT_BPM: f32 = 120.0;

/// The signal name that means "the generator this binding declares".
///
/// It is deliberately not a lookup on the bus: [`SynthesizedBus`]'s own
/// `"noise"` is a parameterless stand-in, while a binding always has kind,
/// rate, stream and octaves to say — and a `&str` cannot carry four fields
/// without a grammar to take them apart again.
pub const NOISE_SIGNAL: &str = "noise";

/// The shape a signal is put through before it reaches `range`.
///
/// Four, and four is the number of distinct shapes a monotone `[0,1] -> [0,1]`
/// map has: flat, floor-weighted, peak-weighted, and eased at both ends. A
/// fifth would be a re-parameterisation of one of these — `pow3` is `pow2`
/// with a steeper knee, `cbrt` is `sqrt` with a steeper one — adding a name
/// without adding a behaviour, which is the kind of vocabulary growth an LLM
/// generating against a specification pays for twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Curve {
    /// Identity. The signal as it is. `docs/ir-spec.md` uses this name.
    Lin,
    /// `x^2`. **Emphasises the peak**: the bottom of the signal is flattened
    /// towards the floor, so the param only moves near the top. This is what
    /// a percussive binding wants — a `beat` pulse through `pow2` reads as a
    /// hit rather than as a wobble. `docs/ir-spec.md` uses this name.
    Pow2,
    /// `sqrt(x)`. **Emphasises the floor**, and is the exact complement of
    /// [`Curve::Pow2`]: it rises fast off zero and flattens at the top, so a
    /// signal that spends its life near the bottom — which every low-confidence
    /// invented signal does, once confidence has scaled it — still produces
    /// visible movement, and the top compresses instead of clipping.
    Sqrt,
    /// Smoothstep, `x^2 (3 - 2x)`. **Eases both ends**: the derivative is zero
    /// at 0 and at 1, so the param neither jumps off the floor nor slams into
    /// the ceiling. The one to use when the param is a position rather than an
    /// intensity, where a discontinuity in velocity is visible and a
    /// discontinuity in brightness is not.
    Smooth,
}

/// Every curve name, in the order they are documented. One list, so the
/// parser, the error message and the spec cannot drift apart.
pub const CURVES: [Curve; 4] = [Curve::Lin, Curve::Pow2, Curve::Sqrt, Curve::Smooth];

impl Curve {
    /// The name a `bind` record spells. `None` for anything else — an
    /// unrecognised curve is a diagnostic, not a silent fallback to `lin`.
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

    /// Apply the curve. The input is clamped into `[0, 1]` first, so the
    /// output is in `[0, 1]` for every curve and `range` is a range rather
    /// than a suggestion. `sqrt` of a negative would be NaN otherwise, which
    /// is one bad sample away from a NaN in a uniform.
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

/// The session's signal source: one local oscillator, one seed, and whatever
/// the record stream last said was measured.
///
/// **One per session.** `Deck` owns it and advances it once per frame; see the
/// module doc. Everything a binding can read comes from here, so a binding's
/// whole input is `(bpm, elapsed steps, seed, this frame's measurements)` and
/// nothing else — no clock, no interior mutability, nothing thread-derived. The
/// last of those is plain data that is *handed in* once per frame, exactly as
/// `steps` is: see [`Signals::set_audio`].
///
/// `Copy`, so that a caller can take the session's signals, put this frame's
/// measurement on them, and hand them back without the phase moving.
#[derive(Clone, Copy)]
pub struct Signals {
    oscillator: Oscillator,
    /// The explicit seed every noise stream is derived from. The determinism
    /// invariant is "all randomness comes from an explicit seed stream", and
    /// this is that seed for the signal side.
    seed: u64,
    /// This frame's measured signals, or `None` when nothing is measuring.
    ///
    /// `None` is not a case any consumer sees: it decides which bus is built
    /// below, and a `None` builds one that answers every name exactly as it did
    /// before audio existed. Someone has to know whether a provider exists —
    /// the invariant is that it is not the consumer, and it is not the bus's
    /// callers.
    audio: Option<AudioFrame>,
}

impl Default for Signals {
    /// [`DEFAULT_BPM`], seed 0, phase zero. What a `Deck` comes up with, and
    /// what a caller driving a `Set` with no bindings on it passes: the bus is
    /// still complete and still answers every name, and nothing reads it.
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

    /// Correct the session's tempo and phase — a new tempo, and a phase shift
    /// in beats.
    ///
    /// **Once per frame at most, before the frame is rendered**, from the same
    /// place `steps` and the measured frame come from: a tracker live, a
    /// `tempo` record on replay. Nothing is measured here and no clock is read;
    /// two numbers arrive and the oscillator applies them, which is what keeps
    /// "rendering reads only the local oscillator" true while an external tempo
    /// source exists at all.
    pub fn correct(&mut self, bpm: f32, shift_beats: f32) {
        self.oscillator.correct(bpm, shift_beats);
    }

    pub fn oscillator(&self) -> &Oscillator {
        &self.oscillator
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Sample a signal by name. Never fails, never returns an `Option`.
    ///
    /// The bus is constructed per call and holds the oscillator and the frame
    /// by reference, so there is nothing to keep in sync and nothing to
    /// allocate — this is a few words on the stack, which matters because it is
    /// called per binding per frame on the render thread.
    ///
    /// Two layers: whatever was measured this frame, over the synthesized bus.
    /// A measured name answers with its own confidence; every other name — and
    /// every name at all, when nothing is measuring — falls through unchanged.
    /// This is the whole of "a binding starts working when audio lands":
    /// `energy` is the same name, sampled by the same call, and only the
    /// confidence that comes back is different.
    pub fn sample(&self, name: &str) -> Sample {
        MeasuredBus::new(self.audio.as_ref(), SynthesizedBus::new(&self.oscillator)).sample(name)
    }

    /// Sample a generator the caller declares, mapped from the generator's
    /// signed `[-1, 1)` into the `[0, 1]` a curve and a range expect.
    ///
    /// **Certain**, unlike the bus's parameterless `"noise"` name. A noise
    /// binding is not a guess at something unobserved: the binding declares
    /// the generator, the generator is deterministic, and its value is exactly
    /// what it claims to be — the same reason the local oscillator's own
    /// signals are certain. `docs/ir-spec.md`'s Spawn timing rests on this:
    /// irregular spawning is available *only* by binding noise to
    /// `spawn_rate`, and a binding that took a tenth effect would not be an
    /// alternative to the Poisson option that section rejects.
    pub fn noise(&self, config: &NoiseConfig) -> Sample {
        Sample::certain(config.sample(self.seed, &self.oscillator) * 0.5 + 0.5)
    }
}

/// One signal, attached to one `param` of one layer.
///
/// Field for field a `Record::Bind`, so that loading a Set file is a decode
/// rather than a translation.
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    /// Which procedure declares the `param`. L1 and L4 params are packed into
    /// separate uniform buffers, so this is what says which one is written.
    pub layer: Kind,
    pub key: String,
    pub signal: String,
    pub curve: Curve,
    pub range: [f32; 2],
    /// The generator, for a `signal` of [`NOISE_SIGNAL`]. `None` means the
    /// default generator, not the absence of one: there is nothing else for the
    /// name to mean.
    pub noise: Option<NoiseConfig>,
    /// What the last [`Binding::resolve`] produced. Cached rather than
    /// recomputed per read because a param is written into two places in a
    /// frame — the uniform and, for `spawn_rate`, the spawn accumulator — and
    /// two evaluations of one binding in one frame is two values.
    value: f32,
}

impl Binding {
    pub fn new(
        layer: Kind,
        key: impl Into<String>,
        signal: impl Into<String>,
        curve: Curve,
        range: [f32; 2],
    ) -> Binding {
        Binding {
            layer,
            key: key.into(),
            signal: signal.into(),
            curve,
            range,
            noise: None,
            // Overwritten by the first `resolve`, which happens in `prepare`
            // before anything reads it. Not `NaN`: a value that leaked would
            // then poison a whole render target rather than being visibly odd.
            value: range[0],
        }
    }

    pub fn with_noise(mut self, noise: NoiseConfig) -> Binding {
        self.noise = Some(noise);
        self
    }

    /// What this binding wrote on the last frame.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Sample, curve, map, and blend against `manual`. Returns what the param
    /// is written with this frame, and remembers it.
    ///
    /// `manual` is the param's own value — its `.kir` default, or whatever a
    /// `param` record or a `--param` override last set. It is never
    /// overwritten: a binding blends *from* it, so a param that is both bound
    /// and set by hand has an answer that does not depend on which of the two
    /// happened last.
    pub fn resolve(&mut self, signals: &Signals, manual: f32) -> f32 {
        let sample = self.sample(signals);
        self.value = blend(manual, self.map(sample.value), sample.confidence);
        self.value
    }

    /// The curve and the range, without the confidence blend. Public so that
    /// "what this binding would write if the signal were certain" is a value a
    /// test can name rather than a number copied out of an implementation.
    pub fn map(&self, x: f32) -> f32 {
        let (low, high) = (self.range[0], self.range[1]);
        low + (high - low) * self.curve.apply(x)
    }

    /// Where the value comes from. A `signal` of [`NOISE_SIGNAL`] reads the
    /// generator the binding carries rather than the bus's parameterless
    /// `"noise"` name: the bus takes a `&str` and there is no collision-free
    /// grammar for four fields inside one, which is exactly why the record
    /// carries a `noise` object instead. Every other name goes to the bus, and
    /// the bus answers every name.
    fn sample(&self, signals: &Signals) -> Sample {
        if self.signal == NOISE_SIGNAL {
            signals.noise(&self.noise.unwrap_or_default())
        } else {
            signals.sample(&self.signal)
        }
    }
}

/// `lerp(manual, mapped, confidence)` — the whole of "consumers branch only on
/// confidence".
///
/// Written as a weighted sum rather than `manual + (mapped - manual) * c` so
/// that both ends are exact: a confidence of 1.0 writes `mapped` bit for bit
/// and a confidence of 0.0 writes `manual` bit for bit, which is what makes
/// "an unknown signal leaves the param alone" a thing to assert rather than to
/// approximate.
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

    /// The rule most likely to be quietly dropped. `energy` is invented and
    /// carries 0.1, so **from the same sample value** it must move a param a
    /// tenth as far as a provider that is tracking would.
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

    /// **One name, one meaning.** `docs/ir-spec.md` requires two vocabularies
    /// that meet in one decoder to be "disjoint by name" rather than merely
    /// disjoint in practice. A binding's `signal` is one name resolved two
    /// ways — [`NOISE_SIGNAL`] reads the generator the binding declares,
    /// everything else reads the bus — so for the rule to hold, no name may be
    /// answerable by both. What a binding writes has to be what the bus says
    /// that name is, for every name the bus provides.
    #[test]
    fn no_signal_name_means_one_thing_to_the_bus_and_another_to_a_binding() {
        let signals = advanced(120.0, 42, 37);
        let manual = 0.5;

        // Every name the bus provides resolves through the bus, unchanged.
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

        // And the one name that does not is not on the bus at all. A binding
        // reads its declared generator, certain; if the bus answered the same
        // name it would answer with a different number at a different
        // confidence, and which one a caller got would depend on which door it
        // came in by.
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

    /// A name no provider has ever heard of. Not an error, not a panic, and
    /// not a change: the param keeps its manual value, bit for bit.
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

    /// Both ends of the blend are exact, which is what lets the two tests
    /// above assert equality rather than a tolerance.
    #[test]
    fn confidence_one_writes_the_mapped_value_and_zero_writes_the_manual_one() {
        assert_eq!(blend(2.6, 9.1, 1.0), 9.1);
        assert_eq!(blend(2.6, 9.1, 0.0), 2.6);
        assert_eq!(blend(2.0, 4.0, 0.5), 3.0);
        // A confidence outside [0, 1] cannot extrapolate past either end.
        assert_eq!(blend(2.6, 9.1, 5.0), 9.1);
        assert_eq!(blend(2.6, 9.1, -5.0), 2.6);
        assert_eq!(blend(2.6, 9.1, f32::NAN), 2.6);
    }

    /// A param that is bound *and* set by hand. The manual value is the blend
    /// base and nothing else, so the answer does not depend on which of the
    /// two was written last.
    #[test]
    fn a_manual_value_is_the_base_of_the_blend_not_a_competitor_for_the_write() {
        let signals = advanced(120.0, 3, 17);
        let mut certain = binding("beat", Curve::Lin, [0.0, 4.0]);
        // Two very different manual values, one certain signal: the write is
        // the same either way, because confidence 1.0 leaves the base no
        // weight at all.
        assert_eq!(
            certain.resolve(&signals, 0.5),
            certain.resolve(&signals, 400.0)
        );

        // And with an invented signal the manual value is most of the answer,
        // so moving it moves the write — a `--param` on a bound param is not
        // ignored, it is outvoted in proportion.
        let mut invented = binding("energy", Curve::Lin, [0.0, 4.0]);
        let a = invented.resolve(&signals, 0.5);
        let b = invented.resolve(&signals, 1.5);
        assert!(
            (b - a - 0.9).abs() < 1e-5,
            "a 1.0 move in a param at confidence 0.1 should move the write by 0.9, moved {}",
            b - a
        );
    }

    // -- phase --------------------------------------------------------------

    /// A `beat` binding moves *in time with the oscillator*, not merely over
    /// time. Two things are asserted, because either alone would pass against
    /// a number that changes for the wrong reason: the value is the beat
    /// signal at the session's own phase, and it repeats a beat later.
    #[test]
    fn a_beat_binding_tracks_the_oscillators_phase() {
        // 120 bpm at dt = 1/60 is exactly 30 frames to the beat.
        let bpm = 120.0;
        let frames_per_beat = 30;
        let mut signals = Signals::new(bpm, 1);
        let mut b = binding("beat", Curve::Lin, [0.0, 8.0]);

        let mut values = Vec::new();
        for _ in 0..(frames_per_beat * 3) {
            signals.advance(1, DT);
            let manual = 3.0;
            let got = b.resolve(&signals, manual);
            // It is the beat signal at this instant, mapped — `beat` is
            // certain, so the manual value has no weight.
            assert_eq!(got, b.map(signals.sample("beat").value));
            values.push(got);
        }

        // It moves at all...
        let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(max - min > 4.0, "a beat binding barely moved: {min}..{max}");

        // ...and it moves *periodically*, at the oscillator's period rather
        // than at some rate of its own.
        for i in 0..frames_per_beat {
            let a = values[i];
            let b = values[i + frames_per_beat];
            assert!(
                (a - b).abs() < 1e-3,
                "frame {i} and one beat later differ: {a} vs {b}"
            );
        }

        // The peak lands *on the beat*, and it is the top of the range. A
        // sample is taken after the advance, so the frame with phase zero is
        // the last of each window of 30 rather than the first.
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
        // Not exactly 8.0: `t` is a sum of `steps * dt` and `dt` is 1/60,
        // which is not representable, so the frame the beat lands on is a few
        // ULP short of phase zero rather than on it.
        assert!(
            (peak_value - 8.0).abs() < 1e-4,
            "the beat instant should reach the top of the range, reached {peak_value}"
        );
    }

    // -- determinism --------------------------------------------------------

    /// The same tick sequence and the same seed reproduce every bound value
    /// bit for bit, noise included. A binding is part of the record stream's
    /// output now, so it is inside the determinism invariant.
    #[test]
    fn the_same_ticks_and_seed_reproduce_every_bound_value_bit_for_bit() {
        // Deliberately ragged: different step counts per frame, so that a
        // binding that depended on the call count rather than on elapsed time
        // would have to agree with one that did not.
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

        // And the noise bindings are actually seeded rather than constant, or
        // the equality above would be worth nothing.
        let other = run(19_275);
        assert_ne!(a, other, "changing the seed changed nothing");
    }

    /// The noise generator is reachable, all of it: two bindings differing
    /// only in `stream` decorrelate, and one with no `noise` object is the
    /// default generator rather than a dead signal.
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

    /// Noise is certain, so a `spawn_rate` bound to it actually spans its
    /// range — which is the whole of the ir-spec's argument against baking a
    /// Poisson distribution into the engine.
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
        // A tenth-effect binding starting from a manual 0.0 could not exceed
        // 0.1 at all, let alone span more than that. One-dimensional perlin
        // does not use its whole amplitude, so the bar is where a tenth
        // becomes impossible rather than at full scale.
        assert!(
            min > 0.1 && max - min > 0.3,
            "a noise binding spanned only {min}..{max} of [0, 1], which a \
             tenth-effect binding could have produced"
        );
    }

    /// Signed noise reaches the whole range rather than sitting on the floor
    /// for the half of its life it spends below zero.
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

    /// `bpm` is not a unit-range signal and a binding to it saturates. Asserted
    /// rather than fixed: an invented normalisation range would be a number
    /// nobody could justify, and `docs/ir-spec.md` says to bind `beat` instead.
    #[test]
    fn a_bpm_binding_saturates_because_bpm_is_not_a_unit_range_signal() {
        let signals = advanced(128.0, 0, 10);
        let mut b = binding("bpm", Curve::Lin, [0.0, 4.0]);
        assert_eq!(b.resolve(&signals, 1.0), 4.0);
    }
}
