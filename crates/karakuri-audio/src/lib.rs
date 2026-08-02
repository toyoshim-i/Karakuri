//! Audio input: a microphone becomes signals on the bus and a correction to
//! the local oscillator.
//!
//! Four parts, and the split is the point — three of them are pure functions
//! and only the fourth knows a device exists:
//!
//! | | |
//! |---|---|
//! | [`analysis`] | samples in, one frame of measured signals out |
//! | [`tempo`] | novelty in, a tempo and a beat phase out, in the octave the grid is already in |
//! | [`lock`] | an estimate in, a correction for the local oscillator out |
//! | [`device`] | opens a stream, runs the first two on it, hands the results across a thread |
//!
//! Everything above the device line is testable against synthesised input — a
//! tone at a known frequency, a click train at a known tempo, silence — which
//! is how this was built and how it is checked, since a machine's audio input
//! is not something a test can rely on.
//!
//! ## What reaches the engine
//!
//! Nothing from here reaches the engine except **values**: an
//! [`AudioFrame`](karakuri_signal::AudioFrame) of measured signals, and a
//! [`Correction`](lock::Correction) for the oscillator. Both are the payloads
//! of records — `audio` and `tempo` — so the live path and a replay hand the
//! engine the same two things and it cannot tell which it is running. That is
//! the arrangement `tick` already has, and it is the only way live audio and a
//! reproducible record stream can both be true.
//!
//! ## The two lags
//!
//! A correction has to lead, or the beat lands late and looks wrong rather than
//! random. The two durations it leads by are:
//!
//! - **A** = input buffer + half an analysis window. Both are known and both
//!   are measured here: [`device::Reading::age`] carries their sum, plus
//!   however long ago the last publish was.
//! - **D** = the frame queue, the present, and the display's own pipeline. The
//!   first is knowable and the last is not, so it is an **operator-adjustable
//!   offset with a default**, not a measurement — see `karakuri-cli`'s
//!   `--display-latency-ms`. A performer will nudge it by ear, and that dial is
//!   the honest place for what cannot be measured.
//!
//! [`lock`] explains what is done with them.

pub mod analysis;
pub mod device;
pub mod lock;
pub mod tempo;

pub use analysis::{Analysis, Analyzer};
pub use device::{staleness, AudioError, AudioInput, Reading};
pub use lock::{BeatLock, Correction, Reason};
pub use tempo::{fold, tracking_window, Estimate, Tracker};
