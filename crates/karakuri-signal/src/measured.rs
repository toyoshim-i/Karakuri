//! Measured signals: one frame's worth of them, and the bus layer that answers
//! from it.
//!
//! Everything in [`bus`](crate::bus) is invented — a waveform in the local
//! oscillator's `t`. This is the other half: values that came from something
//! outside, carried as **plain data with a confidence**, so that the thing
//! producing them (an audio analyser, a replayed record) is not visible from
//! here and is not visible to a consumer either.
//!
//! ## Why this is a value and not a provider object
//!
//! An [`AudioFrame`] is `Copy`, fixed-size, and holds no handle to anything. It
//! is the payload of one `audio` record, field for field, the same way
//! `karakuri-engine`'s `Binding` is a `Record::Bind` field for field. That is
//! what puts audio inside the determinism invariant instead of beside it: the
//! engine never talks to a device, it is *handed a frame* — derived from real
//! input when live, read back off the record stream on replay — exactly as it
//! is handed a `steps` count rather than measuring one. Nothing here reads a
//! clock, allocates, or locks; a frame is about seventy bytes on the stack.
//!
//! ## What confidence means here
//!
//! - **1.0** — a device is open and a block of samples was analysed recently.
//!   A silent room is this: `energy` at 0.0 with confidence 1.0 means *there is
//!   silence*, which is a measurement and moves a bound parameter all the way
//!   to the bottom of its range.
//! - **falling** — the last block is going stale. Whoever assembled the frame
//!   decides how fast (see `karakuri-audio`'s `staleness`), because that is a
//!   measurement of elapsed real time and this crate never reads a clock.
//! - **0.0** — nothing has arrived. The blend then writes the parameter's own
//!   value, bit for bit, which is how an interface unplugged mid-set hands the
//!   parameter back to the operator instead of freezing it at whatever the last
//!   block happened to say.
//!
//! **No frame at all** is a fourth case and it is not the same as any of the
//! three: with no audio configured, [`MeasuredBus`] is handed `None` and every
//! name falls through to the synthesized bus, so a run without a microphone
//! answers exactly what it answered before this module existed, bit for bit.
//!
//! ## What is measured, and what is not
//!
//! The names are the ones the bus already has — `energy`, `band0`, `band1`, …
//! — because a measured `energy` and an invented one are the same signal from
//! different sources, and a binding that has to be rewritten when a microphone
//! appears would defeat the whole arrangement. One name is new, `onset`, and
//! the synthesized bus deliberately does **not** invent a version of it: an
//! invented onset would be `beat`'s pulse under a second name, and one name has
//! to mean one thing. With no provider, `onset` answers 0.0 at confidence 0.0
//! like any other name nobody provides, and a binding to it leaves its
//! parameter alone. Bind `beat` if a pulse with no microphone behind it is what
//! is wanted.

use crate::bus::SynthesizedBus;
use crate::{Sample, SignalBus, SignalId};

/// The signal name for a broadband level. Already a bus name.
pub const ENERGY: &str = "energy";

/// The signal name for a transient onset envelope.
///
/// Decaying envelope in `[0.0, 1.0]`. Jumps to 1.0 on a detected transient and
/// decays with a fixed half-life, ensuring transient visibility across frame boundaries.
pub const ONSET: &str = "onset";

/// How many spectrum bands a frame can carry.
///
/// Eight, log-spaced: enough that a kick, a snare and a hi-hat land in
/// different ones, few enough that each band still has bins in it at a block
/// size a transient survives. The count is part of the vocabulary rather than a
/// dial — `band0`…`band7` are *names*, and a binding written against `band5`
/// should not silently mean a different octave because someone retuned an
/// analyser.
pub const MAX_BANDS: usize = 8;

/// One frame's worth of measured signals.
///
/// Fixed-size and `Copy` on purpose: this is handed across a thread boundary
/// and installed on the session's signals once per frame, on the render thread,
/// where an allocation is not allowed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioFrame {
    /// Broadband level, `[0, 1]`. See `karakuri-audio` for what maps to 1.0.
    pub energy: f32,
    /// Transient envelope, `[0, 1]`. See [`ONSET`].
    pub onset: f32,
    /// Per-band level, `[0, 1]`, low to high. Only the first `band_count` are
    /// meaningful; the rest are zero and are not answered.
    pub bands: [f32; MAX_BANDS],
    /// How many of `bands` were measured. A band index past this is not a
    /// measured signal, so it falls through to whatever is beneath the measured
    /// layer rather than reading a zero that nobody produced.
    pub band_count: u8,
    /// How much of this to believe, `[0, 1]`. See the module doc.
    pub confidence: f32,
}

impl AudioFrame {
    /// A frame that measured silence, at full confidence.
    ///
    /// This is what a live analyser produces from a block of zeroes, and it is
    /// not the same value as [`AudioFrame::nothing`] — the difference between
    /// a quiet room and a dead input is the whole reason confidence is a number
    /// rather than a flag.
    pub fn silent(band_count: u8) -> AudioFrame {
        AudioFrame {
            energy: 0.0,
            onset: 0.0,
            bands: [0.0; MAX_BANDS],
            band_count: band_count.min(MAX_BANDS as u8),
            confidence: 1.0,
        }
    }

    /// A frame from a provider that exists and has said nothing yet: all zeroes
    /// at confidence 0.0, so every bound parameter keeps its own value.
    pub fn nothing(band_count: u8) -> AudioFrame {
        AudioFrame {
            confidence: 0.0,
            ..AudioFrame::silent(band_count)
        }
    }

    /// The same measurement, believed less. What staleness does to a frame; the
    /// values are untouched because they are still what was measured — only the
    /// claim that they are *current* weakens.
    pub fn with_confidence(self, confidence: f32) -> AudioFrame {
        AudioFrame {
            confidence: clamp_unit(confidence),
            ..self
        }
    }

    /// Whether this frame is the one that answers `id`, and with what.
    pub fn provides_id(&self, id: SignalId) -> Option<Sample> {
        let value = match id {
            SignalId::Energy => self.energy,
            SignalId::Onset => self.onset,
            SignalId::Band(index) => {
                if usize::from(index) >= usize::from(self.band_count) {
                    return None;
                }
                self.bands[usize::from(index)]
            }
            _ => return None,
        };
        Some(Sample {
            value: clamp_unit(value),
            confidence: clamp_unit(self.confidence),
        })
    }

    /// Whether this frame is the one that answers `name`, and with what.
    ///
    /// The `Option` is a **provider's** question — "is this one of mine?" — and
    /// never reaches a consumer: [`MeasuredBus::sample`] turns a `None` into
    /// the layer beneath, and that layer answers every name. The bus is still
    /// complete and still returns a `Sample` rather than an `Option`.
    pub fn provides(&self, name: &str) -> Option<Sample> {
        self.provides_id(SignalId::resolve(name))
    }
}

/// A NaN in a measurement is a broken analyser, and a NaN in a confidence is a
/// parameter written with a NaN — one bad block would take a whole render
/// target with it. Clamped at the boundary, once, where measured values enter
/// the system.
fn clamp_unit(x: f32) -> f32 {
    if x.is_nan() {
        0.0
    } else {
        x.clamp(0.0, 1.0)
    }
}

/// The measured layer, over the synthesized one.
///
/// Every name a frame measured is answered from the frame; every other name —
/// including a band index past what was measured, and every oscillator-derived
/// signal — falls through unchanged. With `measured` as `None` this *is* the
/// synthesized bus, which is what keeps a run with no audio configured
/// bit-identical to one from before audio existed.
pub struct MeasuredBus<'a> {
    measured: Option<&'a AudioFrame>,
    beneath: SynthesizedBus<'a>,
}

impl<'a> MeasuredBus<'a> {
    pub fn new(measured: Option<&'a AudioFrame>, beneath: SynthesizedBus<'a>) -> MeasuredBus<'a> {
        MeasuredBus { measured, beneath }
    }
}

impl SignalBus for MeasuredBus<'_> {
    fn sample(&self, name: &str) -> Sample {
        self.sample_id(SignalId::resolve(name))
    }

    fn sample_id(&self, id: SignalId) -> Sample {
        self.measured
            .and_then(|frame| frame.provides_id(id))
            .unwrap_or_else(|| self.beneath.sample_id(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oscillator::Oscillator;

    /// Every name that could possibly be involved, measured or not.
    const NAMES: [&str; 10] = [
        "bpm", "beat", "bar", "energy", "onset", "band", "band0", "band3", "band7", "noise",
    ];

    fn frame() -> AudioFrame {
        AudioFrame {
            energy: 0.4,
            onset: 0.75,
            bands: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
            band_count: 4,
            confidence: 1.0,
        }
    }

    #[test]
    fn a_measured_signal_answers_with_its_own_value_at_full_confidence() {
        let osc = Oscillator::new(128.0);
        let f = frame();
        let bus = MeasuredBus::new(Some(&f), SynthesizedBus::new(&osc));

        assert_eq!(bus.sample("energy"), Sample::certain(0.4));
        assert_eq!(bus.sample("onset"), Sample::certain(0.75));
        // `band` with no index is band 0, the same rule the synthesized bus
        // uses — one name, one meaning, whichever layer answers it.
        assert_eq!(bus.sample("band"), Sample::certain(0.1));
        assert_eq!(bus.sample("band3"), Sample::certain(0.4));
    }

    /// The property the whole layering exists for: adding a provider must not
    /// change the answer to any name it does not provide.
    #[test]
    fn every_unmeasured_name_answers_exactly_what_it_did_without_a_provider() {
        let mut osc = Oscillator::new(128.0);
        osc.advance(37, 1.0 / 60.0);
        let f = frame();
        let measured = MeasuredBus::new(Some(&f), SynthesizedBus::new(&osc));
        let synthesized = SynthesizedBus::new(&osc);

        for name in NAMES {
            let is_measured =
                matches!(name, "energy" | "onset") || matches!(name, "band" | "band0" | "band3");
            if is_measured {
                continue;
            }
            assert_eq!(
                measured.sample(name),
                synthesized.sample(name),
                "`{name}` is not measured, so the measured layer must not touch it"
            );
        }

        // Specifically: a band past what was measured is *not* a measured zero.
        // Four bands were measured, so `band7` is still the invented waveform
        // at invented confidence.
        assert_eq!(measured.sample("band7"), synthesized.sample("band7"));
        assert!(measured.sample("band7").confidence > 0.0);
        assert!(measured.sample("band7").confidence < 0.5);
    }

    /// With no frame at all the layer is transparent, name for name, bit for
    /// bit — a run with no audio configured is the run it was before.
    #[test]
    fn with_no_frame_the_measured_layer_is_the_synthesized_bus() {
        let mut osc = Oscillator::new(97.0);
        osc.advance(11, 1.0 / 60.0);
        let measured = MeasuredBus::new(None, SynthesizedBus::new(&osc));
        let synthesized = SynthesizedBus::new(&osc);
        for name in NAMES {
            assert_eq!(measured.sample(name), synthesized.sample(name), "`{name}`");
        }
    }

    /// A silent room and a dead input are the same numbers and different
    /// confidences, and that difference is the whole point.
    #[test]
    fn silence_is_zero_at_full_confidence_and_nothing_is_zero_at_none() {
        let silent = AudioFrame::silent(8);
        assert_eq!(silent.provides("energy"), Some(Sample::certain(0.0)));
        assert_eq!(silent.provides("band5"), Some(Sample::certain(0.0)));

        let nothing = AudioFrame::nothing(8);
        assert_eq!(nothing.provides("energy"), Some(Sample::synthesized(0.0)));
    }

    /// Staleness weakens the claim without editing the measurement.
    #[test]
    fn confidence_can_be_lowered_without_the_values_moving() {
        let stale = frame().with_confidence(0.25);
        assert_eq!(stale.energy, frame().energy);
        assert_eq!(stale.provides("energy").expect("measured").confidence, 0.25);
        // And a nonsense confidence cannot escape the unit range.
        assert_eq!(frame().with_confidence(4.0).confidence, 1.0);
        assert_eq!(frame().with_confidence(f32::NAN).confidence, 0.0);
    }

    /// `noise` is the `bind` record's name, not a bus name, and a measured
    /// layer must not become the second meaning the bus refused to give it.
    #[test]
    fn the_measured_layer_does_not_answer_noise_either() {
        let osc = Oscillator::new(120.0);
        let f = frame();
        let bus = MeasuredBus::new(Some(&f), SynthesizedBus::new(&osc));
        assert_eq!(bus.sample("noise"), Sample::synthesized(0.0));
    }

    /// A broken analyser cannot put a NaN or an out-of-range value into a
    /// uniform through this door.
    #[test]
    fn a_measurement_outside_the_unit_range_is_clamped_at_the_boundary() {
        let f = AudioFrame {
            energy: f32::NAN,
            onset: 9.0,
            bands: [-1.0; MAX_BANDS],
            band_count: 8,
            confidence: 1.0,
        };
        assert_eq!(f.provides("energy").expect("measured").value, 0.0);
        assert_eq!(f.provides("onset").expect("measured").value, 1.0);
        assert_eq!(f.provides("band2").expect("measured").value, 0.0);
    }
}
