//! The signal bus and the local oscillator.
//!
//! Two invariants shape this crate, and both exist so that consumers never have
//! to know where a value came from:
//!
//! - **The bus is always complete.** A signal with no provider returns a
//!   synthesised value, so consumers never branch on whether a provider exists.
//!   They branch only on [`Sample::confidence`]. A *measured* signal arrives as
//!   one more layer over that same completeness — see [`measured`] — and
//!   changes no answer to any name it does not measure.
//! - **Rendering reads only the local oscillator**, never an external clock.
//!   External input is correction applied to the oscillator, not a substitute
//!   for it.
//!
//! The IR cannot read this bus at all. A procedure reacts to audio or tempo by
//! declaring a `param` and having a `bind` record attach a signal to it, which
//! is what keeps every external coupling declarative.

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

/// A signal value and how much it should be trusted.
///
/// `confidence` is 1.0 for a real provider tracking well, and falls towards 0.0
/// as a value becomes synthesised, stale, or extrapolated. It is never a flag:
/// a consumer blends on it rather than testing it.
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

/// Distributed to every layer. Never fails, never returns an option.
pub trait SignalBus {
    /// Sample a signal by name. An unknown name returns a synthesised value
    /// rather than an error, because the alternative is consumers that branch
    /// on existence.
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
