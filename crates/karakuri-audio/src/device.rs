//! The device: the only part of this crate that talks to hardware.
//!
//! Deliberately thin. Everything that decides anything — [`crate::analysis`],
//! [`crate::tempo`], [`crate::lock`] — is a pure function of samples or of
//! numbers, and is tested against synthesised input. This module opens a
//! stream, feeds those, and hands the result across a thread boundary.
//!
//! ## Where the analysis happens, and why it is on the audio side
//!
//! **In the audio callback.** The alternative — copying raw samples into a ring
//! and transforming them on the render thread — was rejected on two counts: it
//! puts an FFT and an autocorrelation inside the frame budget the engine's
//! governor is policing, and it makes the amount of work per
//! frame depend on how much audio happened to arrive, which is the shape of a
//! frame-time spike.
//!
//! Doing it here is bounded and cheap. **Measured, on a host clock, release
//! build, Apple M4 Pro**: one 2048-sample analysis is 12.7 µs against a hop
//! that carries 10.7 ms of audio, and a tempo estimate — an autocorrelation
//! over every candidate lag, plus one pass folding the window onto the settled
//! period — costs 30 µs once per 256 ms. That is about 0.13% of the callback's
//! time. Both numbers come from the ignored measurements in `analysis` and
//! `tempo`, which print them on demand rather than asserting them, and both
//! move by a third between a cold run and a warm one: the estimator that folds
//! and the two heuristics it replaced measure the same to within that noise. It allocates nothing (every buffer is planned at construction) and
//! locks nothing.
//!
//! ## How the frame reads it without waiting
//!
//! One [`Mutex`] holding one small `Copy` value, and **both sides use
//! `try_lock`**:
//!
//! - the callback skips publishing if the render thread happens to hold it —
//!   losing one measurement out of ninety a second, which staleness already
//!   describes;
//! - the render thread keeps the value it read last frame if the callback
//!   happens to hold it.
//!
//! Neither side ever blocks on the other, which is the requirement in both
//! directions: no audio callback may block on anything, and nothing on the
//! render thread may block on audio. A lock-free seqlock would remove the
//! word "skip" from that paragraph and add fifty lines of ordering argument to
//! this file; the value it protects is stale within 11 ms anyway.
//!
//! One thing travels the other way — the tempo tracker's window centre, which
//! is the grid's current tempo — and it is an `AtomicU32` rather than a second
//! mutex for the reason above: the callback reads it on **every** hop, so
//! "skip on contention" would mean the octave window occasionally not moving,
//! and one `f32` needs no more than a relaxed load to carry.
//!
//! ## Confidence and staleness
//!
//! The analyser says what it measured, at confidence 1.0 — including a
//! measured silence. [`staleness`] is what turns "how long ago" into "how much
//! to believe", and it lives here because it is the one part of the audio path
//! that reads a clock. That is the same division
//! `docs/principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md`
//! draws for `tick`:
//! measurement happens where the clock is, and what comes out joins the record
//! stream.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

/// How much to believe a measurement that was taken `age` ago.
///
/// Full until [`FRESH_SECONDS`], then linear to zero at [`STALE_SECONDS`]. A
/// pure function of a duration, so the whole staleness policy is testable
/// without a device and without waiting.
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
    /// **A**, plus however long ago the last publish was: how far in the past
    /// the instant the estimate describes is, right now. The caller adds the
    /// output lag to get the lead. See [`crate::lock`].
    pub age: f32,
}

struct Published {
    frame: AudioFrame,
    estimate: Estimate,
    /// **A**: half an analysis window, plus whatever of the device buffer
    /// arrived after the sample this block ends on. Measured at publish.
    ///
    /// **Short by the delivery delay** — the gap between the last sample being
    /// captured and the callback running — which nothing portable measures.
    /// That residue and every other unmeasurable term end up in one place: the
    /// operator's offset. See the crate doc's "The two lags".
    analysis_lag: f32,
    at: Instant,
}

/// An open input stream.
pub struct AudioInput {
    /// Held because dropping it closes the stream. Nothing calls it.
    _stream: cpal::Stream,
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
    /// Open an input. `selector` is `"default"`, or any substring of a device's
    /// description or id — case-insensitive, first match wins.
    ///
    /// `centre_bpm` is where the tempo tracker's window starts: the session
    /// tempo, which is also what the oscillator free-runs at. See
    /// [`crate::tempo`].
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

        let mut analyzer = Analyzer::new(sample_rate);
        let bands = analyzer.band_count();
        let window_lag = analyzer.window_lag();
        let mut tracker = Tracker::new(analyzer.hop_seconds(), window_lag, centre_bpm);
        let shared: Arc<Mutex<Option<Published>>> = Arc::new(Mutex::new(None));
        let publisher = Arc::clone(&shared);
        // One `f32` in an atomic rather than a second mutex: the callback reads
        // it on every hop and must never wait, and a torn read is impossible
        // for a `u32`. Relaxed is enough — it orders nothing else, and a centre
        // that arrives one hop late costs at most one re-measurement in the
        // octave the grid was already in.
        let centre_bpm = Arc::new(AtomicU32::new(centre_bpm.to_bits()));
        let follower = Arc::clone(&centre_bpm);

        // Every buffer the callback touches, allocated here. The callback is
        // real-time code: an allocation in it is a lock in disguise.
        let mut ring = vec![0.0f32; BLOCK];
        let mut block = vec![0.0f32; BLOCK];
        let mut write = 0usize;
        let mut filled = 0usize;
        let mut since_hop = 0usize;

        // `frames` is how many this callback delivered and `rate` its sample
        // rate, so that a hop landing part-way through a buffer can say how
        // much of that buffer came *after* it — see the lag below.
        let mut on_samples =
            move |mono: &mut dyn Iterator<Item = f32>, frames: usize, rate: f32| {
                for (index, sample) in mono.enumerate() {
                    ring[write] = sample;
                    write = (write + 1) % BLOCK;
                    filled = (filled + 1).min(BLOCK);
                    since_hop += 1;
                    if since_hop < HOP || filled < BLOCK {
                        continue;
                    }
                    since_hop = 0;

                    // Oldest first, so the newest sample is the last one — which is
                    // the instant every phase in this crate is measured against.
                    let (tail, head) = ring.split_at(write);
                    block[..head.len()].copy_from_slice(head);
                    block[head.len()..].copy_from_slice(tail);

                    let analysis = analyzer.analyze(&block);
                    tracker.set_centre_bpm(f32::from_bits(follower.load(Ordering::Relaxed)));
                    tracker.push(analysis.novelty);

                    if let Ok(mut slot) = publisher.try_lock() {
                        // **How much of this buffer arrived after the sample that
                        // ends this block**, which is how old the analysis instant
                        // already is when it is published. A hop can land anywhere
                        // in a buffer, so this runs from a whole buffer down to
                        // nothing; charging the buffer's *whole* duration every
                        // time — which is what this did — over-stated `A` by up to
                        // one buffer and by half of one on average, and the
                        // correction then led by that much too much.
                        //
                        // What is left unaccounted is the delivery delay: the gap
                        // between the last sample being captured and this callback
                        // running. Nothing portable measures it, and it is one of
                        // the terms the operator's offset exists to absorb.
                        let after = (frames - 1 - index) as f32 / rate;
                        *slot = Some(Published {
                            frame: analysis.frame,
                            estimate: tracker.estimate(),
                            analysis_lag: after + window_lag,
                            at: Instant::now(),
                        });
                    }
                }
            };

        let error = |e: cpal::Error| {
            // Printed rather than propagated: by the time this fires the stream
            // is running, and the frame path's answer to a broken device is
            // already correct — confidence decays and the bindings hand their
            // parameters back.
            eprintln!("audio: {e}");
        };

        // One arm per sample format the host might hand us. The conversion is
        // the same in each; only the type differs.
        macro_rules! stream {
            ($sample:ty) => {
                device.build_input_stream(
                    config.clone(),
                    move |data: &[$sample], _: &cpal::InputCallbackInfo| {
                        // From what actually arrived rather than from the
                        // requested buffer size, because a host is free to
                        // ignore that.
                        let frames = data.len() / channels.max(1);
                        // Downmix: a level is a level, and a stereo input whose
                        // channels differ is not two measurements.
                        let mut mono = data.chunks(channels.max(1)).map(|frame| {
                            frame
                                .iter()
                                .map(|s| <f32 as cpal::FromSample<$sample>>::from_sample_(*s))
                                .sum::<f32>()
                                / channels.max(1) as f32
                        });
                        on_samples(&mut mono, frames, sample_rate as f32);
                    },
                    error,
                    None,
                )
            };
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

        Ok(AudioInput {
            _stream: stream,
            shared,
            centre_bpm,
            last: None,
            description,
            sample_rate,
            bands,
        })
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Tell the tracker where the grid is, so its octave window follows.
    ///
    /// **The grid's tempo, every frame** — that is what makes a drifting tempo
    /// stay in one octave, and it is how the ×2 and ÷2 controls reach the
    /// tracker: they move the grid, and this carries the window along with it.
    /// Never waits; see the module doc for why this side of the boundary is an
    /// atomic and the other is a mutex.
    pub fn set_centre_bpm(&self, bpm: f32) {
        self.centre_bpm.store(bpm.to_bits(), Ordering::Relaxed);
    }

    /// This frame's reading. **Never waits**; see the module doc.
    pub fn read(&mut self) -> Reading {
        if let Ok(mut slot) = self.shared.try_lock() {
            if let Some(published) = slot.take() {
                self.last = Some(published);
            }
        }

        let Some(published) = &self.last else {
            // A device is open and has said nothing yet. Zeroes at confidence
            // 0.0 — every bound parameter keeps its own value, and the grid
            // free-runs.
            return Reading {
                frame: AudioFrame::nothing(self.bands),
                estimate: Estimate::unknown(0.0),
                age: 0.0,
            };
        };

        // **Since the publish, not since the last read.** A reader that reset
        // this would make a dead device look alive for as long as anything kept
        // asking, which is the failure this whole confidence arrangement exists
        // to avoid — and it is the one thing here a device is not needed to
        // check, so it is `aged`'s to answer and a test's to hold.
        aged(published, published.at.elapsed())
    }
}

/// What one publish reads as, `since` after it was published.
///
/// Split out because everything above it needs a device and none of this does:
/// it is the arithmetic that turns "how long ago" into "how much to believe"
/// and into the lead the caller adds the output lag to.
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
        // **A**, plus however long ago the publish was. Not multiplied by
        // anything: how old the measurement is and how much it is believed are
        // two different statements, and the lead needs the first one whole.
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

fn describe(device: &cpal::Device) -> String {
    device
        .description()
        .map(|d| d.name().to_string())
        .ok()
        .or_else(|| device.id().ok().map(|id| id.to_string()))
        .unwrap_or_else(|| "unnamed input".to_string())
}

/// **What input devices there are**, by the description [`AudioInput::open`]
/// matches a selector against and [`AudioInput::description`] hands back — so
/// a name from this list, passed back to `open`, opens that device.
///
/// # A list that exists only inside a refusal is not a list
///
/// This enumeration was here before this function was, and the only way to
/// reach it was to ask for a device that is not there and read
/// [`AudioError::NoMatch`]'s `available`. That is fine for the operator who
/// mistyped `--audio-in`, which is what it was written for, and it is not a
/// listing: **a menu cannot be drawn from the text of an error.** A panel that
/// wanted to offer the room's inputs would have had to open a device it did
/// not want, under a name it made up, in the hope of being turned down — and
/// then parse the sentence it was turned down with. The refusal is a sentence
/// for a person; this is a value for a program, and the two now come out of
/// one enumeration rather than agreeing by accident.
///
/// **It reads the host every call and caches nothing.** An interface plugged
/// in between two calls is a different answer, and it should be: the caller
/// that asks is a hand about to open a menu, which is exactly the moment the
/// answer has to be current. It is not a thing to ask on a frame path
/// (`docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md`),
/// for the same reason a directory listing is not.
///
/// **Empty is a room with no microphone, and it is not a failure** — see
/// `docs/principles/0034-a-quiet-room-is-not-a-missing-microphone.md`. Nothing
/// here decides what to do about that, because deciding is the caller's: a
/// machine with no input is a machine where every name answers what it
/// answered before audio existed.
pub fn inputs() -> Vec<String> {
    named(&enumerated(&cpal::default_host()))
}

/// Every input this host offers, in the host's own order. **One enumeration**,
/// held rather than repeated: [`pick`] needs the devices to match against and
/// the names to refuse with, and asking the host twice for one press is how
/// the two answers came to be able to disagree.
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

    /// **The list a menu would draw is the list a refusal carries.**
    ///
    /// [`inputs`] exists because the enumeration used to be reachable only by
    /// being turned down, and the whole point of naming it is that the two
    /// are one answer. So the check is that they *are* one: ask for a device
    /// that cannot be there, and what the refusal names is what the listing
    /// names.
    ///
    /// **It needs no device and it is not a test that passes because nothing
    /// ran.** A machine with no inputs answers `[]` on both sides and the
    /// assertion still bites — it compares two derivations, not a list
    /// against a length. A machine with inputs compares the names. What it
    /// cannot check either way is that a stream opens, which is what no test
    /// in this crate checks and why everything above [`AudioInput`] is a pure
    /// function.
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

    /// **A device that has stopped delivering reaches zero, and takes
    /// [`STALE_SECONDS`] to get there.** The values do not move — they are
    /// still what was measured — and both confidences fall together, because
    /// they came out of one block at one instant.
    ///
    /// This is the arithmetic behind the difference between a quiet room and a
    /// dead input, which is otherwise only checkable by unplugging something.
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
