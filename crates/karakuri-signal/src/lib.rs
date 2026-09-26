//! Signal bus and local oscillator for tempo and modulation routing.
//! Connects synthesized clocks, audio measurements, and procedural noise
//! to parameter bindings via deterministic signal IDs.

pub mod bus;
pub mod id;
pub mod measured;
pub mod noise;
pub mod oscillator;

pub use bus::SynthesizedBus;
pub use id::{SignalId, SignalValue, VectorSample};
pub use measured::{AudioFrame, MeasuredBus, MAX_BANDS};
pub use noise::{NoiseConfig, NoiseKind};
pub use oscillator::Oscillator;

/// A signal value paired with a confidence rating in `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub value: f32,
    pub confidence: f32,
}

impl Sample {
    /// A value from a provider that is tracking.
    pub fn certain(value: f32) -> Sample {
        Sample {
            value,
            confidence: 1.0,
        }
    }

    /// A value with no provider behind it. Still a usable number — that is the
    /// point of the bus being complete.
    pub fn synthesized(value: f32) -> Sample {
        Sample {
            value,
            confidence: 0.0,
        }
    }
}

/// Interface for querying signals without fallibility.
pub trait SignalBus {
    /// Samples a signal by name, returning a synthesized fallback if unrecognized.
    fn sample(&self, name: &str) -> Sample {
        self.sample_id(SignalId::resolve(name))
    }

    /// Sample a signal by its pre-resolved ID. Zero string hashing or comparisons
    /// on the hot path.
    fn sample_id(&self, id: SignalId) -> Sample;

    /// Sample a vectorized signal by its ID. Defaults to wrapping the scalar
    /// sample from [`sample_id`](SignalBus::sample_id).
    fn sample_vector(&self, id: SignalId) -> VectorSample {
        let s = self.sample_id(id);
        VectorSample {
            value: SignalValue::Scalar(s.value),
            confidence: s.confidence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Empty;

    impl SignalBus for Empty {
        fn sample_id(&self, _id: SignalId) -> Sample {
            Sample::synthesized(0.0)
        }
    }

    #[test]
    fn an_unknown_signal_still_yields_a_value() {
        // The bus is complete: there is no "missing" case for a consumer to
        // handle, only a low-confidence one.
        let s = Empty.sample("no_such_signal");
        assert_eq!(s.value, 0.0);
        assert_eq!(s.confidence, 0.0);

        let s_id = Empty.sample_id(SignalId::Custom(1234));
        assert_eq!(s_id.value, 0.0);
        assert_eq!(s_id.confidence, 0.0);

        let vs = Empty.sample_vector(SignalId::Custom(1234));
        assert_eq!(vs.confidence, 0.0);
        assert_eq!(vs.value, SignalValue::Scalar(0.0));
    }
}
