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
//! - **A** = half an analysis window, plus whatever of the device buffer
//!   arrived after the sample the block ends on. Both are known and both are
//!   measured here: [`device::Reading::age`] carries their sum, plus however
//!   long ago the last publish was.
//! - **D** = the frame queue, the present, and everything past the two outputs.
//!   The first is knowable and the rest is not.
//!
//! [`lock`] explains what is done with them.
//!
//! ## Why the offset is the answer and not a better measurement
//!
//! It is tempting to read the unknown part of **D** as the display's own
//! pipeline and go looking for a number for it. That is the wrong shape of the
//! problem. **Sound and picture leave by different paths and neither ends at
//! the machine**: the audio goes to a desk, through processing, to a PA that
//! may be metres or tens of metres from the audience and may be delayed
//! deliberately; the picture goes to a projector, a scaler, an LED processor,
//! a stream. Either can be later. Nothing at this end can see any of it, and
//! the sum is regularly larger than everything measured here put together.
//!
//! It also cannot be measured *at* the machine even in principle, because what
//! has to line up is what a person in the room sees and hears — a position, not
//! a signal. So the honest instrument is the one an operator uses: play sound
//! and picture, stand where the audience stands, and move an offset until they
//! land together.
//!
//! Everything here therefore aims at one thing — that the estimate be
//! **stable**, so the offset stays put once it is found. An estimate that is
//! wrong by a fixed amount costs one adjustment; an estimate that drifts costs
//! the operator the whole night. That is why `A` is computed rather than
//! rounded to the buffer it arrived in, and why the residue it cannot see is
//! named rather than guessed at: a constant unknown is absorbed by the offset,
//! and a varying one is not.
//!
//! The offset is `karakuri-cli`'s `--latency-offset-ms`, on the `o` and `p`
//! keys, and it is **signed** — the picture is as often the early one as the
//! late one.

pub mod analysis;
pub mod device;
pub mod lock;
pub mod tempo;

pub use analysis::{Analysis, Analyzer};
pub use device::{inputs, staleness, AudioError, AudioInput, Reading};
pub use lock::{BeatLock, Correction, Reason};
pub use tempo::{fold, tracking_window, Estimate, Tracker};
