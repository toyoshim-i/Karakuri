//! Real-time audio input capture, spectral analysis, and tempo tracking.
//!
//! Subsystems:
//! - [`analysis`]: Transforms PCM sample blocks into spectral frames and onsets.
//! - [`tempo`]: Evaluates spectral novelty to track tempo and beat phase within an octave window.
//! - [`lock`]: Phase-locked loop emitting phase and drift corrections for clock oscillators.
//! - [`device`]: Manages hardware audio streams via CPAL and publishes non-blocking measurements.
//!
//! Output models ([`karakuri_signal::AudioFrame`] and [`Correction`]) are pure values recorded
//! into session journals, ensuring identical offline replay behavior without audio hardware.
//!
//! Latency handling tracks internal analysis latency (windowing and buffer age) while
//! supporting external calibration offsets for acoustic and display pipeline delays.

pub mod analysis;
pub mod device;
pub mod lock;
pub mod tempo;

pub use analysis::{Analysis, Analyzer};
pub use device::{inputs, staleness, AudioError, AudioInput, Reading};
pub use lock::{BeatLock, Correction, Reason};
pub use tempo::{fold, tracking_window, Estimate, Tracker};
