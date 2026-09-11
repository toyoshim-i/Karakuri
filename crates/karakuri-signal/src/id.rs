//! Signal identifiers and vectorized signal representations.
//!
//! Provides zero-lookup dispatch through pre-resolved [`SignalId`] enums,
//! and typed signal representations ([`SignalValue`], [`VectorSample`]).

use crate::Sample;

/// Pre-allocated names for standard spectrum bands 0 through 15.
const BAND_NAMES: [&str; 16] = [
    "band0", "band1", "band2", "band3", "band4", "band5", "band6", "band7", "band8", "band9",
    "band10", "band11", "band12", "band13", "band14", "band15",
];

/// A typed signal identifier.
///
/// Eliminates string lookups, string comparisons, and hashing on the per-frame hot path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalId {
    Bpm,
    Beat,
    Bar,
    Energy,
    Onset,
    Band(u8),
    Custom(u16),
}

impl SignalId {
    /// Resolve a signal name to a typed [`SignalId`].
    ///
    /// Maps:
    /// - `"bpm"` -> [`SignalId::Bpm`]
    /// - `"beat"` -> [`SignalId::Beat`]
    /// - `"bar"` -> [`SignalId::Bar`]
    /// - `"energy"` -> [`SignalId::Energy`]
    /// - `"onset"` -> [`SignalId::Onset`]
    /// - `"band"` -> [`SignalId::Band(0)`]
    /// - `"band0"`..`"band15"` (and any decimal `u8`) -> [`SignalId::Band(i)`]
    /// - Unknown names -> [`SignalId::Custom(hash)`]
    pub fn resolve(name: &str) -> SignalId {
        match name {
            "bpm" => SignalId::Bpm,
            "beat" => SignalId::Beat,
            "bar" => SignalId::Bar,
            "energy" => SignalId::Energy,
            "onset" => SignalId::Onset,
            "band" => SignalId::Band(0),
            _ => {
                if let Some(rest) = name.strip_prefix("band") {
                    if let Ok(i) = rest.parse::<u8>() {
                        return SignalId::Band(i);
                    }
                }
                SignalId::Custom(fnv1a_hash_u16(name.as_bytes()))
            }
        }
    }

    /// The canonical static string name for standard signals, if one exists.
    pub const fn name(&self) -> Option<&'static str> {
        match *self {
            SignalId::Bpm => Some("bpm"),
            SignalId::Beat => Some("beat"),
            SignalId::Bar => Some("bar"),
            SignalId::Energy => Some("energy"),
            SignalId::Onset => Some("onset"),
            SignalId::Band(i) => {
                if (i as usize) < BAND_NAMES.len() {
                    Some(BAND_NAMES[i as usize])
                } else {
                    None
                }
            }
            SignalId::Custom(_) => None,
        }
    }
}

impl std::fmt::Display for SignalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = self.name() {
            write!(f, "{name}")
        } else {
            match self {
                SignalId::Band(i) => write!(f, "band{i}"),
                SignalId::Custom(h) => write!(f, "custom:{h:04x}"),
                _ => unreachable!(),
            }
        }
    }
}

/// 16-bit FNV-1a hash for interning unknown signal names into [`SignalId::Custom`].
const fn fnv1a_hash_u16(bytes: &[u8]) -> u16 {
    let mut hash: u32 = 0x811c_9dc5;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u32;
        hash = hash.wrapping_mul(0x0100_0193);
        i += 1;
    }
    ((hash >> 16) ^ (hash & 0xffff)) as u16
}

/// Vectorized and typed signal payload.
#[derive(Debug, Clone, PartialEq)]
pub enum SignalValue {
    Scalar(f32),
    Vec4([f32; 4]),
    Spectrum(Vec<f32>),
}

impl SignalValue {
    /// Return the primary scalar representation, if applicable.
    pub fn as_scalar(&self) -> Option<f32> {
        match self {
            SignalValue::Scalar(v) => Some(*v),
            SignalValue::Vec4(v) => Some(v[0]),
            SignalValue::Spectrum(v) => v.first().copied(),
        }
    }

    /// Return contiguous slice of component floats.
    pub fn as_slice(&self) -> &[f32] {
        match self {
            SignalValue::Scalar(v) => std::slice::from_ref(v),
            SignalValue::Vec4(v) => v.as_slice(),
            SignalValue::Spectrum(v) => v.as_slice(),
        }
    }
}

/// A vectorized signal value paired with a confidence metric.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorSample {
    pub value: SignalValue,
    pub confidence: f32,
}

impl VectorSample {
    /// Construct a new vector sample with value and confidence.
    pub fn new(value: SignalValue, confidence: f32) -> Self {
        VectorSample { value, confidence }
    }

    /// Construct a scalar vector sample.
    pub fn scalar(value: f32, confidence: f32) -> Self {
        VectorSample {
            value: SignalValue::Scalar(value),
            confidence,
        }
    }

    /// Construct a 4-component vector sample.
    pub fn vec4(value: [f32; 4], confidence: f32) -> Self {
        VectorSample {
            value: SignalValue::Vec4(value),
            confidence,
        }
    }

    /// Construct a spectrum buffer vector sample.
    pub fn spectrum(value: impl Into<Vec<f32>>, confidence: f32) -> Self {
        VectorSample {
            value: SignalValue::Spectrum(value.into()),
            confidence,
        }
    }

    /// Convert into a traditional scalar [`Sample`].
    pub fn to_sample(&self) -> Sample {
        Sample {
            value: self.value.as_scalar().unwrap_or(0.0),
            confidence: self.confidence,
        }
    }
}

impl From<Sample> for VectorSample {
    fn from(s: Sample) -> Self {
        VectorSample::scalar(s.value, s.confidence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_maps_standard_signals() {
        assert_eq!(SignalId::resolve("bpm"), SignalId::Bpm);
        assert_eq!(SignalId::resolve("beat"), SignalId::Beat);
        assert_eq!(SignalId::resolve("bar"), SignalId::Bar);
        assert_eq!(SignalId::resolve("energy"), SignalId::Energy);
        assert_eq!(SignalId::resolve("onset"), SignalId::Onset);
        assert_eq!(SignalId::resolve("band"), SignalId::Band(0));
        assert_eq!(SignalId::resolve("band0"), SignalId::Band(0));
        assert_eq!(SignalId::resolve("band3"), SignalId::Band(3));
        assert_eq!(SignalId::resolve("band15"), SignalId::Band(15));
    }

    #[test]
    fn resolve_maps_unknown_to_custom() {
        let id1 = SignalId::resolve("my_custom_signal");
        let id2 = SignalId::resolve("my_custom_signal");
        let id3 = SignalId::resolve("another_signal");

        match id1 {
            SignalId::Custom(hash) => {
                assert_eq!(id1, id2);
                assert_ne!(id1, id3);
                assert_ne!(hash, 0);
            }
            _ => panic!("expected Custom variant"),
        }

        // Near-misses like "banding" are Custom, not Band
        match SignalId::resolve("banding") {
            SignalId::Custom(_) => {}
            other => panic!("expected Custom, got {:?}", other),
        }
    }

    #[test]
    fn name_returns_canonical_strings() {
        assert_eq!(SignalId::Bpm.name(), Some("bpm"));
        assert_eq!(SignalId::Beat.name(), Some("beat"));
        assert_eq!(SignalId::Bar.name(), Some("bar"));
        assert_eq!(SignalId::Energy.name(), Some("energy"));
        assert_eq!(SignalId::Onset.name(), Some("onset"));
        assert_eq!(SignalId::Band(0).name(), Some("band0"));
        assert_eq!(SignalId::Band(15).name(), Some("band15"));
        assert_eq!(SignalId::Band(16).name(), None);
        assert_eq!(SignalId::Custom(42).name(), None);
    }

    #[test]
    fn vector_sample_conversions() {
        let s = Sample::certain(0.85);
        let vs: VectorSample = s.into();
        assert_eq!(vs.confidence, 1.0);
        assert_eq!(vs.value, SignalValue::Scalar(0.85));
        assert_eq!(vs.to_sample(), s);

        let v4 = VectorSample::vec4([1.0, 2.0, 3.0, 4.0], 0.9);
        assert_eq!(v4.value.as_scalar(), Some(1.0));
        assert_eq!(v4.value.as_slice(), &[1.0, 2.0, 3.0, 4.0]);

        let spec = VectorSample::spectrum(vec![0.1, 0.2, 0.3], 0.5);
        assert_eq!(spec.value.as_scalar(), Some(0.1));
        assert_eq!(spec.value.as_slice(), &[0.1, 0.2, 0.3]);
    }
}
