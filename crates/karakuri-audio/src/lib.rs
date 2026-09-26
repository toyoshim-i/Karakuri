//! Real-time audio input capture, spectral analysis, and tempo tracking.
//! Captures hardware PCM streams, performs spectral feature extraction,
//! and synchronizes tempo tracking with deterministic journal logging.

pub mod analysis;
pub mod device;
pub mod lock;
pub mod tempo;

pub use analysis::{Analysis, Analyzer};
pub use device::{inputs, staleness, AudioError, AudioInput, Reading};
pub use lock::{BeatLock, Correction, Reason};
pub use tempo::{fold, tracking_window, Estimate, Tracker};
