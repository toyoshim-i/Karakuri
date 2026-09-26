//! Measured audio signal payload and bus routing.
//! Encapsulates external audio analysis features ([`AudioFrame`]) as plain data with
//! associated confidence ratings and zero allocations.

use crate::bus::SynthesizedBus;
use crate::{Sample, SignalBus, SignalId};

/// The signal name for a broadband level. Already a bus name.
pub const ENERGY: &str = "energy";

/// The signal name for a transient onset envelope.
///
/// Decaying envelope in `[0.0, 1.0]`. Jumps to 1.0 on a detected transient and
/// decays with a fixed half-life, ensuring transient visibility across frame boundaries.
pub const ONSET: &str = "onset";

/// The signal name for sub-bass energy band (band 0, ~40-85 Hz).
pub const SUB: &str = "sub";
/// The signal name for bass energy band (band 1, ~85-180 Hz).
pub const BASS: &str = "bass";
/// The signal name for mid energy band (band 3, ~380-800 Hz).
pub const MID: &str = "mid";
/// The signal name for high/air energy band (band 7, ~7.6-16 kHz).
pub const AIR: &str = "air";

/// Maximum spectrum bands supported in a single audio frame (log-spaced).
pub const MAX_BANDS: usize = 8;

/// One frame's worth of measured audio signals.
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
    /// Band index for sub-bass (~40–85 Hz).
    pub const SUB: usize = 0;
    /// Band index for bass (~85–180 Hz).
    pub const BASS: usize = 1;
    /// Band index for low-mid (~180–380 Hz).
    pub const LOW_MID: usize = 2;
    /// Band index for mid (~380–800 Hz).
    pub const MID: usize = 3;
    /// Band index for high-mid (~800–1700 Hz).
    pub const HIGH_MID: usize = 4;
    /// Band index for presence (~1700–3600 Hz).
    pub const PRESENCE: usize = 5;
    /// Band index for brilliance (~3600–7500 Hz).
    pub const BRILLIANCE: usize = 6;
    /// Band index for air / high (~7500–16000 Hz).
    pub const AIR: usize = 7;

    /// Sub-bass level (`bands[0]`, ~40–85 Hz).
    pub fn sub(&self) -> f32 {
        self.bands[Self::SUB]
    }

    /// Bass level (`bands[1]`, ~85–180 Hz).
    pub fn bass(&self) -> f32 {
        self.bands[Self::BASS]
    }

    /// Low-mid level (`bands[2]`, ~180–380 Hz).
    pub fn low_mid(&self) -> f32 {
        self.bands[Self::LOW_MID]
    }

    /// Mid-range level (`bands[3]`, ~380–800 Hz).
    pub fn mid(&self) -> f32 {
        self.bands[Self::MID]
    }

    /// High-mid level (`bands[4]`, ~800–1700 Hz).
    pub fn high_mid(&self) -> f32 {
        self.bands[Self::HIGH_MID]
    }

    /// Presence level (`bands[5]`, ~1700–3600 Hz).
    pub fn presence(&self) -> f32 {
        self.bands[Self::PRESENCE]
    }

    /// Brilliance level (`bands[6]`, ~3600–7500 Hz).
    pub fn brilliance(&self) -> f32 {
        self.bands[Self::BRILLIANCE]
    }

    /// Air / high level (`bands[7]`, ~7500–16000 Hz).
    pub fn air(&self) -> f32 {
        self.bands[Self::AIR]
    }

    /// Returns a frame representing measured silence with full confidence (1.0).
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

    /// Evaluates whether this frame provides the named signal, returning its sample if present.
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

/// Layered signal bus overlaying measured audio frame samples onto synthesized defaults.
///
/// Measured frame channels override underlying values; unmeasured channels and
/// oscillator signals fall through to the underlying [`SynthesizedBus`].
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
        // Semantic band names
        assert_eq!(bus.sample("sub"), Sample::certain(0.1));
        assert_eq!(bus.sample("audio.sub"), Sample::certain(0.1));
        assert_eq!(bus.sample("bass"), Sample::certain(0.2));
        assert_eq!(bus.sample("audio.bass"), Sample::certain(0.2));
        assert_eq!(bus.sample("low_mid"), Sample::certain(0.3));
        assert_eq!(bus.sample("mid"), Sample::certain(0.4));
        assert_eq!(bus.sample("audio.mid"), Sample::certain(0.4));
    }

    #[test]
    fn audio_frame_named_band_accessors_match_bands() {
        let f = frame();
        assert_eq!(f.sub(), 0.1);
        assert_eq!(f.bass(), 0.2);
        assert_eq!(f.low_mid(), 0.3);
        assert_eq!(f.mid(), 0.4);
        assert_eq!(f.high_mid(), 0.5);
        assert_eq!(f.presence(), 0.6);
        assert_eq!(f.brilliance(), 0.7);
        assert_eq!(f.air(), 0.8);
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
