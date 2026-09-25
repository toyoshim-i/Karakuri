//! Audio input management and beat synchronization for live sessions.
//!
//! Emits audio and tempo records into the session stream and applies measured signals
//! and beat lock adjustments to the engine oscillator.

use std::time::Instant;

use karakuri_audio::lock::BeatLock;

/// Reason for the last tempo correction.
pub use karakuri_audio::lock::Reason;
use karakuri_audio::{AudioInput, Correction};

/// Audio input device open error.
pub use karakuri_audio::AudioError;

/// Enumerates available host audio input devices.
pub use karakuri_audio::inputs;

/// Searchable BPM range for beat tracking.
pub use karakuri_audio::tempo::BPM_RANGE;

use karakuri_engine::Signals;
use karakuri_signal::measured::{AudioFrame, MAX_BANDS};
use karakuri_store::record::Record;

/// Number of frames held in the presentation queue.
const QUEUE_FRAMES: f32 = 2.0;

/// Default baseline display pipeline latency offset in milliseconds.
pub const DEFAULT_LATENCY_OFFSET_MS: f32 = 20.0;

/// Step size for manual latency offset adjustment.
pub const LATENCY_OFFSET_STEP_MS: f32 = 5.0;

/// Valid range for manual latency offset adjustment.
pub const LATENCY_OFFSET_RANGE: std::ops::RangeInclusive<f32> = -200.0..=200.0;

/// Live audio session managing hardware input, beat locking, and latency compensation.
pub struct Audio {
    input: AudioInput,
    lock: BeatLock,
    latency_offset_ms: f32,
    frame_interval: f32,
    last: Status,
    /// Pre-allocated record reused across frames to prevent heap allocation on the render thread.
    audio: Record,
}

/// What a performer needs to see. A tempo they cannot read is a tempo they
/// cannot tell is wrong.
#[derive(Debug, Clone, Copy, Default)]
pub struct Status {
    pub energy: f32,
    pub onset: f32,
    pub confidence: f32,
    pub estimated_bpm: f32,
    pub estimate_confidence: f32,
    /// The tracker's note that the grid may be at half the music's tempo — the
    /// operator's cue to press ×2. Nothing acts on it; see `karakuri-audio`'s
    /// `tempo` module for why that is the whole point.
    pub half_tempo_hint: bool,
    /// Phase error in beats, signed. Positive means the music is ahead of the
    /// picture, so the offset wants raising.
    pub error: f32,
    pub locked: bool,
}

impl Audio {
    /// Open an input. `selector` is `default` or part of a device's name.
    ///
    /// `session_bpm` defines the free-running oscillator BPM and the tempo tracker's
    /// initial octave window center. See `karakuri-audio::tempo`.
    pub fn open(
        selector: &str,
        latency_offset_ms: f32,
        dt: f32,
        session_bpm: f32,
    ) -> Result<Audio, AudioError> {
        Ok(Audio {
            input: AudioInput::open(selector, session_bpm)?,
            lock: BeatLock::new(),
            latency_offset_ms: clamped_latency(latency_offset_ms),
            // Starts at the simulation step and is corrected by measurement
            // within a frame or two.
            frame_interval: dt,
            last: Status::default(),
            // At full capacity from the start, so the first `frame` does not
            // grow it and the render thread never sees a `realloc` either.
            audio: Record::Audio {
                energy: 0.0,
                onset: 0.0,
                bands: Vec::with_capacity(MAX_BANDS),
                confidence: 0.0,
            },
        })
    }

    pub fn description(&self) -> &str {
        self.input.description()
    }

    pub fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }

    pub fn status(&self) -> Status {
        self.last
    }

    pub fn latency_offset_ms(&self) -> f32 {
        self.latency_offset_ms
    }

    /// Nudge the offset. Returns what it became, for printing: a control that
    /// changes something invisible is indistinguishable from a broken one.
    pub fn nudge_latency_offset(&mut self, delta_ms: f32) -> f32 {
        self.latency_offset_ms = clamped_latency(self.latency_offset_ms + delta_ms);
        self.latency_offset_ms
    }

    /// D: the frame queue, plus the operator's offset for everything past the two
    /// outputs. See `karakuri-audio`'s crate doc.
    pub fn output_lag(&self) -> f32 {
        output_lag(self.frame_interval, self.latency_offset_ms)
    }

    /// Processes one frame of audio input, emits records, and updates session signals and beat tracking.
    ///
    /// The audio record is returned by reference and overwritten on the next call.
    /// When `grid` is `Grid::Followed`, beat lock writes to the grid are inhibited.
    pub fn frame(
        &mut self,
        signals: &mut Signals,
        elapsed: f32,
        step: f32,
        grid: Grid,
    ) -> (&Record, Option<Record>) {
        if elapsed > 0.0 && elapsed < 1.0 {
            self.frame_interval += (elapsed - self.frame_interval) * 0.05;
        }

        // Center tracking window on the current oscillator tempo.
        self.input.set_centre_bpm(signals.oscillator().bpm());

        let reading = self.input.read();
        write_audio_record(&mut self.audio, &reading.frame);
        signals.set_audio(audio_frame(&self.audio));

        let ahead = reading.age + self.output_lag();
        let correction = self
            .lock
            .update(&reading.estimate, ahead, signals.oscillator(), step);
        let correction = match grid {
            Grid::Owned => correction,
            Grid::Followed => None,
        };
        let tempo = correction.map(|c| {
            let record = tempo_record(&c);
            apply_tempo(signals, &record);
            record
        });

        self.last = Status {
            energy: reading.frame.energy,
            onset: reading.frame.onset,
            confidence: reading.frame.confidence,
            estimated_bpm: reading.estimate.bpm,
            estimate_confidence: reading.estimate.confidence,
            half_tempo_hint: reading.estimate.half_tempo_hint,
            error: self.lock.error(),
            locked: self.lock.locked(),
        };
        (&self.audio, tempo)
    }

    /// Returns a mutable reference to the frame's audio record for session capture.
    pub fn record_mut(&mut self) -> &mut Record {
        &mut self.audio
    }

    /// Shifts the tempo grid by an octave factor (`2.0` for double, `0.5` for half).
    ///
    /// Returns `None` if the resulting tempo falls outside the trackable range.
    pub fn octave(&mut self, signals: &mut Signals, factor: f32) -> Option<Record> {
        let correction = self.lock.octave(factor, signals.oscillator())?;
        let record = tempo_record(&correction);
        apply_tempo(signals, &record);
        self.input.set_centre_bpm(signals.oscillator().bpm());
        self.last.locked = self.lock.locked();
        self.last.error = self.lock.error();
        Some(record)
    }

    /// Retargets the beat lock and tracker window to a user-specified free-run tempo (ADR-0291).
    pub fn set_tempo(&mut self, bpm: f32) {
        self.lock.retarget(bpm);
        self.input.set_centre_bpm(bpm);
        self.last.locked = self.lock.locked();
        self.last.error = self.lock.error();
    }

    /// A performer tapping the beat. Authoritative, and it goes through the same
    /// record as everything else.
    pub fn tap(&mut self, signals: &mut Signals, at: Instant, since_start: Instant) -> Record {
        let seconds = at.duration_since(since_start).as_secs_f64();
        let correction = self
            .lock
            .tap(seconds, self.output_lag(), signals.oscillator());
        let record = tempo_record(&correction);
        apply_tempo(signals, &record);
        self.last.locked = self.lock.locked();
        self.last.error = self.lock.error();
        record
    }

    /// What the last correction was for, for printing.
    pub fn reason(&self) -> Option<Reason> {
        self.lock.reason()
    }
}

/// Calculates presentation output lag in seconds from queue frame depth and latency offset.
pub fn output_lag(frame_interval: f32, latency_offset_ms: f32) -> f32 {
    QUEUE_FRAMES * frame_interval + latency_offset_ms / 1000.0
}

/// Who is allowed to move the session's grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grid {
    /// This tracker's, because nothing else is on it.
    Owned,
    /// Something else's — a tempo source. The tracker keeps tracking and keeps
    /// quiet.
    Followed,
}

/// Clamp the operator's offset into [`LATENCY_OFFSET_RANGE`].
fn clamped_latency(ms: f32) -> f32 {
    ms.clamp(*LATENCY_OFFSET_RANGE.start(), *LATENCY_OFFSET_RANGE.end())
}

/// Populates an existing `Record::Audio` from an `AudioFrame`, reusing its band buffer without heap allocation.
fn write_audio_record(record: &mut Record, frame: &AudioFrame) {
    let Record::Audio {
        energy,
        onset,
        bands,
        confidence,
    } = record
    else {
        panic!("the reused audio record is not a `Record::Audio`");
    };
    *energy = frame.energy;
    *onset = frame.onset;
    *confidence = frame.confidence;
    bands.clear();
    bands.extend_from_slice(&frame.bands[..usize::from(frame.band_count).min(MAX_BANDS)]);
}

/// Converts a `Record::Audio` back into an `AudioFrame`, or `None` if the record is not an audio frame.
pub fn audio_frame(record: &Record) -> Option<AudioFrame> {
    let Record::Audio {
        energy,
        onset,
        bands,
        confidence,
    } = record
    else {
        return None;
    };
    let count = bands.len().min(MAX_BANDS);
    let mut measured = [0.0f32; MAX_BANDS];
    measured[..count].copy_from_slice(&bands[..count]);
    Some(AudioFrame {
        energy: *energy,
        onset: *onset,
        bands: measured,
        band_count: count as u8,
        confidence: *confidence,
    })
}

/// A correction as the record that carries it.
pub fn tempo_record(correction: &Correction) -> Record {
    Record::Tempo {
        bpm: correction.bpm,
        shift: correction.shift,
        confidence: correction.confidence,
    }
}

/// Apply a `tempo` record to the session. The only way a correction reaches the
/// oscillator, live or on replay.
pub fn apply_tempo(signals: &mut Signals, record: &Record) {
    if let Record::Tempo { bpm, shift, .. } = record {
        signals.correct(*bpm, *shift);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record built from scratch. Tests only, and the allocation is why: the
    /// frame path rewrites one record in place, so a function that returns a fresh
    /// one has no caller there and would be a standing invitation to become one.
    fn audio_record(frame: &AudioFrame) -> Record {
        let mut record = Record::Audio {
            energy: 0.0,
            onset: 0.0,
            bands: Vec::new(),
            confidence: 0.0,
        };
        write_audio_record(&mut record, frame);
        record
    }

    fn frame() -> AudioFrame {
        AudioFrame {
            energy: 0.42,
            onset: 0.75,
            bands: [0.9, 0.4, 0.2, 0.11, 0.05, 0.02, 0.01, 0.0],
            band_count: 8,
            confidence: 1.0,
        }
    }

    /// Verifies that rewriting the audio record reuses its band buffer without allocating.
    #[test]
    fn rewriting_the_audio_record_reuses_its_band_buffer() {
        let mut record = Record::Audio {
            energy: 0.0,
            onset: 0.0,
            bands: Vec::with_capacity(MAX_BANDS),
            confidence: 0.0,
        };
        let Record::Audio { bands, .. } = &record else {
            unreachable!()
        };
        let (pointer, capacity) = (bands.as_ptr(), bands.capacity());

        for count in [3u8, 8, 1, 8, 0, 5] {
            let mut measured = frame();
            measured.band_count = count;
            write_audio_record(&mut record, &measured);
            let Record::Audio { bands, .. } = &record else {
                unreachable!()
            };
            assert_eq!(
                bands.len(),
                usize::from(count),
                "the record does not carry the bands it was given"
            );
            assert_eq!(
                bands.as_ptr(),
                pointer,
                "the band buffer moved at {count} bands, so the frame path allocated"
            );
            assert_eq!(bands.capacity(), capacity, "the band buffer was regrown");
        }
    }

    /// A decoded frame reproduces the values it was emitted with — the whole reason
    /// the measurement is in the stream at all.
    #[test]
    fn a_frame_survives_the_record_and_the_json_between() {
        let record = audio_record(&frame());
        let line = serde_json::to_string(&record).expect("serialise");
        let decoded: Record = serde_json::from_str(&line).expect("parse");
        assert_eq!(audio_frame(&decoded), Some(frame()));
    }

    /// Silence and absence stay different through the round trip, because that
    /// difference is the one an operator reads off a status line when an interface
    /// dies mid-set.
    #[test]
    fn silence_and_absence_survive_as_different_frames() {
        let silent = AudioFrame::silent(8);
        let absent = AudioFrame::nothing(8);
        assert_eq!(audio_frame(&audio_record(&silent)), Some(silent));
        assert_eq!(audio_frame(&audio_record(&absent)), Some(absent));
        assert_ne!(silent, absent);
    }

    /// A frame measuring fewer bands than the vocabulary has keeps its own count,
    /// so the bands nobody measured stay unanswered rather than becoming measured
    /// zeroes.
    #[test]
    fn a_short_band_list_stays_short() {
        let mut short = frame();
        short.band_count = 3;
        short.bands[3..].fill(0.0);
        let back = audio_frame(&audio_record(&short)).expect("an audio record");
        assert_eq!(back.band_count, 3);
        assert_eq!(back, short);
    }

    /// A record that is not an audio record is no measurement, which is a different
    /// thing from a measurement of nothing: it leaves the bus answering `energy`
    /// the way it does with no microphone in the building, rather than answering it
    /// with a zero nobody took.
    #[test]
    fn another_record_is_not_a_measurement_at_all() {
        assert_eq!(audio_frame(&Record::Tick { steps: 1 }), None);

        let mut with_nothing = Signals::new(120.0, 0);
        with_nothing.set_audio(audio_frame(&Record::Tick { steps: 1 }));
        let untouched = Signals::new(120.0, 0);
        assert_eq!(
            with_nothing.sample("energy"),
            untouched.sample("energy"),
            "a non-measurement changed what `energy` answers"
        );

        // Where a measurement *of* nothing does claim the name, at no
        // confidence — which leaves the parameter alone by arithmetic rather
        // than by falling through.
        let measured = audio_frame(&audio_record(&AudioFrame::nothing(8))).expect("a frame");
        assert_eq!(
            measured.provides("energy").expect("claimed").confidence,
            0.0
        );
    }

    /// A correction reaches the oscillator only through the record, so a replay
    /// that applied the same line lands in the same place.
    #[test]
    fn a_correction_reaches_the_oscillator_through_its_record() {
        let mut signals = Signals::new(120.0, 0);
        signals.advance(60, 1.0 / 60.0);
        let before = signals.oscillator().beat_phase();

        let record = tempo_record(&Correction {
            bpm: 128.0,
            shift: 0.25,
            confidence: 0.9,
        });
        apply_tempo(&mut signals, &record);
        assert_eq!(signals.oscillator().bpm(), 128.0);
        assert!(
            (signals.oscillator().beat_phase() - (before + 0.25)).abs() < 1e-5,
            "the shift in the record did not reach the phase"
        );

        // And through the JSON, which is what a replay would actually read.
        let line = serde_json::to_string(&record).expect("serialise");
        let decoded: Record = serde_json::from_str(&line).expect("parse");
        let mut replayed = Signals::new(120.0, 0);
        replayed.advance(60, 1.0 / 60.0);
        apply_tempo(&mut replayed, &decoded);
        assert_eq!(
            replayed.oscillator().beat_phase(),
            signals.oscillator().beat_phase()
        );
        assert_eq!(replayed.oscillator().bpm(), signals.oscillator().bpm());
    }

    /// Verifies latency offset clamping behavior at range limits.
    #[test]
    fn the_offset_moves_the_output_lag_and_stops_at_its_bounds() {
        let mut latency = clamped_latency(DEFAULT_LATENCY_OFFSET_MS + LATENCY_OFFSET_STEP_MS);
        assert_eq!(latency, DEFAULT_LATENCY_OFFSET_MS + LATENCY_OFFSET_STEP_MS);
        for _ in 0..100 {
            latency = clamped_latency(latency - LATENCY_OFFSET_STEP_MS);
        }
        assert_eq!(latency, *LATENCY_OFFSET_RANGE.start());
        for _ in 0..100 {
            latency = clamped_latency(latency + LATENCY_OFFSET_STEP_MS);
        }
        assert_eq!(latency, *LATENCY_OFFSET_RANGE.end());
    }

    /// D itself, which nothing checked: two terms in two different units, summed
    /// into seconds. `Audio` cannot be built without a device, so `output_lag` is a
    /// free function and this is the assertion that says the milliseconds are
    /// divided and the frames are not.
    #[test]
    fn the_output_lag_is_two_frames_of_queue_plus_the_offset_in_seconds() {
        // 60 Hz, no offset at all: exactly two frames.
        assert!((output_lag(1.0 / 60.0, 0.0) - 2.0 / 60.0).abs() < 1e-9);
        // The offset is milliseconds and arrives in seconds.
        assert!((output_lag(0.0, 20.0) - 0.020).abs() < 1e-9);
        // Together, and on the 120 Hz panel this was developed on, where the
        // queue term is half what a `dt`-derived one would have said.
        assert!((output_lag(1.0 / 120.0, 20.0) - (2.0 / 120.0 + 0.020)).abs() < 1e-9);
        assert!(output_lag(1.0 / 120.0, 20.0) < output_lag(1.0 / 60.0, 20.0));
        // And the default lands somewhere a display plausibly is, rather than
        // a thousand times off it in either direction.
        let default = output_lag(1.0 / 60.0, DEFAULT_LATENCY_OFFSET_MS);
        assert!(
            (0.02..0.10).contains(&default),
            "the default output lag is {default} s"
        );
    }

    /// Verifies negative latency offsets allow output lag compensation for delayed sound systems.
    #[test]
    fn the_offset_goes_negative_so_a_late_room_can_be_corrected_for() {
        assert!(
            *LATENCY_OFFSET_RANGE.start() < 0.0,
            "the offset cannot reach a room whose sound is the late one"
        );
        // Turned all the way down, the lead is negative at any frame rate a
        // display runs at — so it is the *sum* that goes below zero, not just
        // the offset term while the queue quietly holds it up.
        let floor = *LATENCY_OFFSET_RANGE.start();
        for hz in [30.0, 60.0, 120.0, 240.0] {
            let lag = output_lag(1.0 / hz, floor);
            assert!(lag < 0.0, "at {hz} Hz the floor still leads by {lag} s");
        }
        // And it is monotone through zero rather than clamped at it, which is
        // the failure this replaces: the control has to keep moving the picture
        // as it is turned down past the point the two outputs agree.
        let steps: Vec<f32> = (-4..=4)
            .map(|n| output_lag(1.0 / 60.0, n as f32 * 25.0))
            .collect();
        assert!(
            steps.windows(2).all(|w| w[0] < w[1]),
            "the lead stopped moving somewhere in {steps:?}"
        );
    }
}
