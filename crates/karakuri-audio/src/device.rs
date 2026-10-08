//! Audio input device stream management and lock-free thread synchronization.
//! Executes frame analysis on callback threads, synchronizing data across
//! thread boundaries via try_lock access with decay on dropped frames.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use web_time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use karakuri_signal::measured::AudioFrame;

use crate::analysis::{Analyzer, BLOCK, HOP};
use crate::tempo::{Estimate, Tracker};

/// How long a measurement stays fully believed. Four analysis hops or so: long
/// enough to ride out a scheduling hiccup, short enough that a dead input is
/// noticed within a frame or two.
pub const FRESH_SECONDS: f32 = 0.05;

/// How long a measurement takes to become worthless. Half a second of falling
/// confidence is a visible settle rather than a snap: an interface unplugged
/// mid-set hands its parameters back to the operator over half a second, and
/// the beat grid free-runs from wherever it was.
pub const STALE_SECONDS: f32 = 0.5;

/// Computes confidence attenuation factor in `[0.0, 1.0]` based on elapsed duration.
pub fn staleness(age: Duration) -> f32 {
    let age = age.as_secs_f32();
    if age <= FRESH_SECONDS {
        1.0
    } else if age >= STALE_SECONDS {
        0.0
    } else {
        1.0 - (age - FRESH_SECONDS) / (STALE_SECONDS - FRESH_SECONDS)
    }
}

/// What one frame reads: the measured signals, the tempo estimate, and how old
/// the estimate's contents are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    /// Measured signals, with staleness already applied to the confidence.
    pub frame: AudioFrame,
    /// The tempo estimate, with staleness applied to *its* confidence too — a
    /// stale estimate must stop being evidence, or the lock would keep trimming
    /// towards a grid nobody is measuring any more.
    pub estimate: Estimate,
    /// Elapsed time since the analysis window midpoint, including buffer age.
    /// Added to output presentation latency to determine overall lead time.
    pub age: f32,
}

struct Published {
    frame: AudioFrame,
    estimate: Estimate,
    /// Analysis window latency (half window duration plus tail buffer lag).
    analysis_lag: f32,
    at: Instant,
}

/// Core audio analysis pipeline decoupled from driver stream handles.
/// Processes mono PCM samples in fixed-size blocks, running spectral analysis,
/// onset detection, and beat tracking.
pub struct AudioCore {
    analyzer: Analyzer,
    tracker: Tracker,
    ring: Vec<f32>,
    block: Vec<f32>,
    write: usize,
    filled: usize,
    since_hop: usize,
    window_lag: f32,
    publisher: Arc<Mutex<Option<Published>>>,
    centre_bpm: Arc<AtomicU32>,
}

impl AudioCore {
    /// Ingests an iterator of mono PCM samples, running spectral analysis and tempo tracking
    /// whenever a hop boundary is reached.
    pub fn feed(&mut self, mono: &mut dyn Iterator<Item = f32>, frames: usize, rate: f32) {
        for (index, sample) in mono.enumerate() {
            self.ring[self.write] = sample;
            self.write = (self.write + 1) % BLOCK;
            self.filled = (self.filled + 1).min(BLOCK);
            self.since_hop += 1;
            if self.since_hop < HOP || self.filled < BLOCK {
                continue;
            }
            self.since_hop = 0;

            let (tail, head) = self.ring.split_at(self.write);
            self.block[..head.len()].copy_from_slice(head);
            self.block[head.len()..].copy_from_slice(tail);

            let analysis = self.analyzer.analyze(&self.block);
            self.tracker
                .set_centre_bpm(f32::from_bits(self.centre_bpm.load(Ordering::Relaxed)));
            self.tracker.push(analysis.novelty);

            if let Ok(mut slot) = self.publisher.try_lock() {
                let after = (frames.saturating_sub(1 + index)) as f32 / rate;
                *slot = Some(Published {
                    frame: analysis.frame,
                    estimate: self.tracker.estimate(),
                    analysis_lag: after + self.window_lag,
                    at: Instant::now(),
                });
            }
        }
    }
}

/// An open input stream.
pub struct AudioInput {
    /// Held because dropping it closes the stream. None when fed from custom sources (e.g. Web Audio API).
    _stream: Option<cpal::Stream>,
    shared: Arc<Mutex<Option<Published>>>,
    /// The tracking window's centre, as `f32` bits, written by the render
    /// thread and read in the callback. See [`AudioInput::set_centre_bpm`].
    centre_bpm: Arc<AtomicU32>,
    /// The most recent publish this reader has managed to pick up. Kept so a
    /// contended `try_lock` costs nothing but a repeated value.
    last: Option<Published>,
    description: String,
    sample_rate: u32,
    bands: u8,
}

impl AudioInput {
    /// Creates an `AudioInput` paired with an `AudioCore` feeder for custom PCM sources
    /// (such as Web Audio API on wasm32 or synthetic signal generators).
    pub fn custom(
        description: impl Into<String>,
        sample_rate: u32,
        centre_bpm: f32,
    ) -> (AudioInput, Arc<Mutex<AudioCore>>) {
        let analyzer = Analyzer::new(sample_rate);
        let bands = analyzer.band_count();
        let window_lag = analyzer.window_lag();
        let tracker = Tracker::new(analyzer.hop_seconds(), window_lag, centre_bpm);
        let shared: Arc<Mutex<Option<Published>>> = Arc::new(Mutex::new(None));
        let publisher = Arc::clone(&shared);
        let centre_bpm_atomic = Arc::new(AtomicU32::new(centre_bpm.to_bits()));
        let follower = Arc::clone(&centre_bpm_atomic);

        let core = AudioCore {
            analyzer,
            tracker,
            ring: vec![0.0f32; BLOCK],
            block: vec![0.0f32; BLOCK],
            write: 0,
            filled: 0,
            since_hop: 0,
            window_lag,
            publisher,
            centre_bpm: follower,
        };

        let input = AudioInput {
            _stream: None,
            shared,
            centre_bpm: centre_bpm_atomic,
            last: None,
            description: description.into(),
            sample_rate,
            bands,
        };

        (input, Arc::new(Mutex::new(core)))
    }

    /// Opens an audio input stream matching `selector` with tracking centered at `centre_bpm`.
    pub fn open(selector: &str, centre_bpm: f32) -> Result<AudioInput, AudioError> {
        let host = cpal::default_host();
        let device = pick(&host, selector)?;
        let description = describe(&device);

        let supported = device
            .default_input_config()
            .map_err(|e| AudioError::Config(e.to_string()))?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let sample_rate = config.sample_rate;
        let channels = config.channels as usize;

        let (mut input, core) = Self::custom(description, sample_rate, centre_bpm);

        let error = |e: cpal::Error| {
            eprintln!("audio: {e}");
        };

        macro_rules! stream {
            ($sample:ty) => {{
                let core_cb = Arc::clone(&core);
                device.build_input_stream(
                    config.clone(),
                    move |data: &[$sample], _: &cpal::InputCallbackInfo| {
                        let frames = data.len() / channels.max(1);
                        let mut mono = data.chunks(channels.max(1)).map(|frame| {
                            frame
                                .iter()
                                .map(|s| <f32 as cpal::FromSample<$sample>>::from_sample_(*s))
                                .sum::<f32>()
                                / channels.max(1) as f32
                        });
                        if let Ok(mut c) = core_cb.lock() {
                            c.feed(&mut mono, frames, sample_rate as f32);
                        }
                    },
                    error,
                    None,
                )
            }};
        }

        let stream = match format {
            cpal::SampleFormat::F32 => stream!(f32),
            cpal::SampleFormat::I16 => stream!(i16),
            cpal::SampleFormat::I32 => stream!(i32),
            cpal::SampleFormat::U16 => stream!(u16),
            cpal::SampleFormat::I8 => stream!(i8),
            cpal::SampleFormat::U8 => stream!(u8),
            other => return Err(AudioError::SampleFormat(format!("{other:?}"))),
        }
        .map_err(|e| AudioError::Build(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AudioError::Build(e.to_string()))?;

        input._stream = Some(stream);
        Ok(input)
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Sets the target grid tempo in BPM to update the tracking octave window.
    pub fn set_centre_bpm(&self, bpm: f32) {
        self.centre_bpm.store(bpm.to_bits(), Ordering::Relaxed);
    }

    /// Reads the latest published audio frame and tempo estimate without blocking.
    pub fn read(&mut self) -> Reading {
        if let Ok(mut slot) = self.shared.try_lock() {
            if let Some(published) = slot.take() {
                self.last = Some(published);
            }
        }

        let Some(published) = &self.last else {
            // Initial state before first publish: return zeroed frame and free-run.
            return Reading {
                frame: AudioFrame::nothing(self.bands),
                estimate: Estimate::unknown(0.0),
                age: 0.0,
            };
        };

        // Age is measured relative to the publish timestamp, not previous read calls.
        aged(published, published.at.elapsed())
    }
}

/// Evaluates a published measurement snapshot after elapsed duration `since`.
fn aged(published: &Published, since: Duration) -> Reading {
    let believed = staleness(since);
    Reading {
        frame: published
            .frame
            .with_confidence(published.frame.confidence * believed),
        estimate: Estimate {
            confidence: published.estimate.confidence * believed,
            ..published.estimate
        },
        // Analysis lag plus elapsed time since publish; measurement age
        // is tracked independently of confidence staleness.
        age: since.as_secs_f32() + published.analysis_lag,
    }
}

/// Everything that can stop an input from opening, with enough in it to act on.
#[derive(Debug)]
pub enum AudioError {
    /// No device matched, and here is what there was. The list *is* the error
    /// message: an operator who typed the wrong name needs the right one, not
    /// an invitation to run a second command for it.
    NoMatch {
        wanted: String,
        available: Vec<String>,
    },
    Config(String),
    Build(String),
    SampleFormat(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::NoMatch { wanted, available } if available.is_empty() => {
                write!(f, "no audio input named `{wanted}`, and no inputs at all")
            }
            AudioError::NoMatch { wanted, available } => {
                write!(f, "no audio input matching `{wanted}`. available:")?;
                for name in available {
                    write!(f, "\n  {name}")?;
                }
                Ok(())
            }
            AudioError::Config(e) => write!(f, "audio input has no usable configuration: {e}"),
            AudioError::Build(e) => write!(f, "could not start the audio input: {e}"),
            AudioError::SampleFormat(e) => write!(f, "unsupported input sample format: {e}"),
        }
    }
}

impl std::error::Error for AudioError {}

/// Fallback description used when a CPAL device name cannot be queried.
fn describe(device: &cpal::Device) -> String {
    device
        .description()
        .map(|d| d.name().to_string())
        .ok()
        .or_else(|| device.id().ok().map(|id| id.to_string()))
        .unwrap_or_else(|| "unnamed input".to_string())
}

/// Returns the list of available audio input device names from the host.
///
/// Enumerates devices directly from the host audio subsystem on demand without caching.
#[cfg(not(target_arch = "wasm32"))]
pub fn inputs() -> Vec<String> {
    named(&enumerated(&cpal::default_host()))
}

#[cfg(target_arch = "wasm32")]
pub fn inputs() -> Vec<String> {
    vec!["Default Microphone".to_string()]
}

/// Enumerates input devices available on the host.
fn enumerated(host: &cpal::Host) -> Vec<cpal::Device> {
    host.input_devices()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
}

/// What those devices are called — the one place a device becomes a name, so
/// [`inputs`] and a refusal's `available` are the same list said twice rather
/// than two lists.
fn named(devices: &[cpal::Device]) -> Vec<String> {
    devices.iter().map(describe).collect()
}

fn pick(host: &cpal::Host, selector: &str) -> Result<cpal::Device, AudioError> {
    let mut devices = enumerated(host);
    if selector.eq_ignore_ascii_case("default") {
        return host
            .default_input_device()
            .ok_or_else(|| AudioError::NoMatch {
                wanted: selector.to_string(),
                available: named(&devices),
            });
    }
    let wanted = selector.to_lowercase();
    // `position` and then a remove, rather than `find`: the list is what the
    // refusal carries, so it has to still be here after the match failed.
    match devices
        .iter()
        .position(|d| describe(d).to_lowercase().contains(&wanted))
    {
        Some(at) => Ok(devices.swap_remove(at)),
        None => Err(AudioError::NoMatch {
            wanted: selector.to_string(),
            available: named(&devices),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three things confidence has to say about time, none of which needs a
    /// device to check.
    #[test]
    fn a_measurement_is_believed_fully_then_less_then_not_at_all() {
        assert_eq!(staleness(Duration::ZERO), 1.0);
        assert_eq!(staleness(Duration::from_secs_f32(FRESH_SECONDS)), 1.0);
        assert_eq!(staleness(Duration::from_secs_f32(STALE_SECONDS)), 0.0);
        assert_eq!(staleness(Duration::from_secs(30)), 0.0);

        // Monotone in between, and actually in between — a step function would
        // pass "full then none" and be a snap on stage.
        let middle = staleness(Duration::from_secs_f32(
            (FRESH_SECONDS + STALE_SECONDS) / 2.0,
        ));
        assert!(
            (0.4..0.6).contains(&middle),
            "half way to stale read {middle}"
        );
        let mut previous = 1.0;
        for ms in 0..600 {
            let now = staleness(Duration::from_millis(ms));
            assert!(now <= previous, "confidence rose again at {ms} ms");
            previous = now;
        }
    }

    /// A frame that measured silence, aged: the values stay zero and only the
    /// belief falls. Which is the distinction the whole design turns on — a
    /// quiet room reads 0.0 at full confidence, a dead input reads 0.0 at none,
    /// and the same arithmetic covers both.
    #[test]
    fn staleness_moves_the_confidence_and_not_the_measurement() {
        let silent = AudioFrame::silent(8);
        let stale =
            silent.with_confidence(silent.confidence * staleness(Duration::from_millis(300)));
        assert_eq!(stale.energy, 0.0);
        assert!(stale.confidence > 0.0 && stale.confidence < 1.0);

        let dead = silent.with_confidence(silent.confidence * staleness(Duration::from_secs(2)));
        assert_eq!(dead.energy, 0.0);
        assert_eq!(dead.confidence, 0.0);
    }

    fn published() -> Published {
        Published {
            frame: AudioFrame {
                energy: 0.6,
                onset: 0.25,
                bands: [0.5; karakuri_signal::MAX_BANDS],
                band_count: 8,
                confidence: 1.0,
            },
            estimate: Estimate {
                bpm: 128.0,
                phase: 0.25,
                confidence: 0.9,
                at: 4.0,
                revision: 7,
                half_tempo_hint: false,
            },
            analysis_lag: 0.032,
            at: Instant::now(),
        }
    }

    /// Verifies that the available device list reported in `AudioError::NoMatch`
    /// matches the enumeration returned by `inputs()`.
    #[test]
    fn the_refusal_names_the_same_inputs_the_listing_does() {
        // Not a substring of any device description anywhere: `pick` matches
        // case-insensitively on `contains`, so the selector has to be
        // something no name can hold.
        let nowhere = "\u{fffd}no such audio input\u{fffd}";
        let error = AudioInput::open(nowhere, 120.0)
            .err()
            .expect("a device by that name cannot exist");
        let AudioError::NoMatch { wanted, available } = &error else {
            panic!("asking for a device that is not there answered `{error}`");
        };
        assert_eq!(wanted, nowhere);
        assert_eq!(
            available,
            &inputs(),
            "the refusal's list and the listing came out of two enumerations"
        );
    }

    /// Verifies that staleness decay reduces confidence to zero over `STALE_SECONDS`
    /// without mutating underlying measured values.
    #[test]
    fn a_publish_that_is_never_replaced_decays_to_nothing_and_says_how_old_it_is() {
        let published = published();
        let mut previous = 1.0;
        for ms in [0u64, 50, 150, 275, 400, 499] {
            let reading = aged(&published, Duration::from_millis(ms));
            assert_eq!(
                reading.frame.energy, 0.6,
                "the measurement moved at {ms} ms"
            );
            assert_eq!(reading.estimate.bpm, 128.0);
            assert!(
                reading.frame.confidence <= previous,
                "confidence rose again at {ms} ms"
            );
            // The estimate is believed in the same proportion, or a stale grid
            // would go on being evidence after the signals stopped being.
            assert!(
                (reading.estimate.confidence
                    - published.estimate.confidence * reading.frame.confidence)
                    .abs()
                    < 1e-6
            );
            // ...and the age is the whole truth about how old it is, including
            // the analysis lag, unscaled by how much it is believed.
            assert!(
                (reading.age - (ms as f32 / 1000.0 + published.analysis_lag)).abs() < 1e-6,
                "age at {ms} ms was {}",
                reading.age
            );
            previous = reading.frame.confidence;
        }

        // Fully believed for the first FRESH_SECONDS, and gone by STALE_SECONDS
        // — a slide over 450 ms rather than a snap, and it does reach zero.
        assert_eq!(aged(&published, Duration::ZERO).frame.confidence, 1.0);
        assert_eq!(
            aged(&published, Duration::from_secs_f32(FRESH_SECONDS))
                .frame
                .confidence,
            1.0
        );
        let dead = aged(&published, Duration::from_secs_f32(STALE_SECONDS));
        assert_eq!(dead.frame.confidence, 0.0);
        assert_eq!(dead.estimate.confidence, 0.0);
        assert_eq!(
            dead.frame.energy, 0.6,
            "a dead input rewrote the last block"
        );
    }
}
