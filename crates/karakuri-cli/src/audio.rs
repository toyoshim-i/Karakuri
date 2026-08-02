//! Audio, from the device to the session — and through the record stream on
//! the way.
//!
//! This is the only place in the CLI that knows a microphone exists, and it is
//! deliberately the same shape as the one place that knows a clock exists: it
//! **measures, emits records, and hands the engine what the records say**. The
//! engine is given an `AudioFrame` and a tempo correction, never a device, so
//! a replay that decoded the same records would hand it the same two things.
//!
//! ```text
//!   device → analyser → tracker ┐
//!                               ├→ Record::Audio ─→ Signals::set_audio
//!                     beat lock ┴→ Record::Tempo ─→ Oscillator::correct
//! ```
//!
//! Every arrow above is a value, and the two records are the payloads, field
//! for field — the same relationship `Binding` has to `Record::Bind`. The live
//! path builds them and reads them back, so the conversion is exercised every
//! frame rather than only by a test.
//!
//! **What is not built here is a session writer.** Nothing in this repository
//! writes a session stream to disk yet, so these records are produced and
//! consumed within a frame. Writing them out, and a replay driver that reads
//! them back instead of opening a device, are the remaining halves — and they
//! are a decoder and a file, not a design.

use std::time::Instant;

use karakuri_audio::lock::{BeatLock, Reason};
use karakuri_audio::{AudioError, AudioInput, Correction};
use karakuri_engine::Signals;
use karakuri_signal::measured::{AudioFrame, MAX_BANDS};
use karakuri_store::record::Record;

/// How many frames the presentation queue holds. `desired_maximum_frame_latency`
/// is 2 where the surface is configured, and this has to agree with it: it is
/// half of the output lag a beat correction leads by.
const QUEUE_FRAMES: f32 = 2.0;

/// The default for the part of the output lag that **cannot be measured** — the
/// display's own pipeline, from the cable to the panel. 20 ms is a reasonable
/// modern monitor and a poor television.
///
/// This is an offset, not a measurement, and it is on a key for that reason: a
/// performer nudging it by ear is the only instrument that can read it.
pub const DEFAULT_DISPLAY_LATENCY_MS: f32 = 20.0;

/// One press of the offset keys.
pub const DISPLAY_LATENCY_STEP_MS: f32 = 5.0;

/// The bounds the offset is held inside. Zero is "the panel is instant", and
/// 200 ms is past any display and well into the region where a performer has
/// mistaken a whole beat for an offset.
pub const DISPLAY_LATENCY_RANGE: std::ops::RangeInclusive<f32> = 0.0..=200.0;

/// The audio session: an open input, the beat lock, and the operator's offset.
pub struct Audio {
    input: AudioInput,
    lock: BeatLock,
    display_latency_ms: f32,
    /// A smoothed frame interval, for the queue part of the output lag. Fed
    /// from the same elapsed-time measurement `steps` comes from — the frame
    /// rate is a property of the machine and the display, and guessing it from
    /// `dt` would be wrong on exactly the 120 Hz panel this was built on.
    frame_interval: f32,
    /// What was last read, for the status line.
    last: Status,
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
    /// `session_bpm` is `--bpm`, and it does **two** jobs: it is what the
    /// oscillator free-runs at, and it is where the tempo tracker's octave
    /// window starts. They are the same number because they are the same
    /// statement — "this is roughly the tempo" — and after the first lock the
    /// window centre simply follows the grid. See `karakuri-audio`'s `tempo`
    /// module.
    pub fn open(
        selector: &str,
        display_latency_ms: f32,
        dt: f32,
        session_bpm: f32,
    ) -> Result<Audio, AudioError> {
        Ok(Audio {
            input: AudioInput::open(selector, session_bpm)?,
            lock: BeatLock::new(),
            display_latency_ms: clamped_latency(display_latency_ms),
            // Starts at the simulation step and is corrected by measurement
            // within a frame or two.
            frame_interval: dt,
            last: Status::default(),
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

    pub fn display_latency_ms(&self) -> f32 {
        self.display_latency_ms
    }

    /// Nudge the offset. Returns what it became, for printing: a control that
    /// changes something invisible is indistinguishable from a broken one.
    pub fn nudge_display_latency(&mut self, delta_ms: f32) -> f32 {
        self.display_latency_ms = clamped_latency(self.display_latency_ms + delta_ms);
        self.display_latency_ms
    }

    /// **D**: the frame queue plus the display's own pipeline. See
    /// `karakuri-audio`'s crate doc.
    pub fn output_lag(&self) -> f32 {
        output_lag(self.frame_interval, self.display_latency_ms)
    }

    /// One frame's worth of audio: read the device, emit the records, and apply
    /// what they say to the session's signals.
    ///
    /// Returns the records, which is what a session writer would take. Nothing
    /// writes them yet — see the module doc — but they are *built here and read
    /// back here*, so the path the engine is driven through is the record's
    /// rather than one that happens to agree with it.
    ///
    /// Never blocks: the device read is a `try_lock` that keeps the previous
    /// value on contention.
    pub fn frame(
        &mut self,
        signals: &mut Signals,
        elapsed: f32,
        step: f32,
    ) -> (Record, Option<Record>) {
        // A frame interval measured on the host clock, smoothed hard: this is
        // an input to a latency, and a single hitched frame is not a change in
        // how deep the queue is.
        if elapsed > 0.0 && elapsed < 1.0 {
            self.frame_interval += (elapsed - self.frame_interval) * 0.05;
        }

        // **Where the tracker looks, every frame.** The grid's tempo is the
        // centre of the one-octave window every candidate period folds into, so
        // a tempo that drifts is followed without anything deciding anything —
        // and the ×2 key works by moving the grid and letting this carry the
        // window with it.
        self.input.set_centre_bpm(signals.oscillator().bpm());

        let reading = self.input.read();
        let audio = audio_record(&reading.frame);
        // Read back rather than used directly. One small allocation per frame
        // in the CLI's own loop — not the engine's, and not a GPU resource —
        // and what it buys is that the live path and a replay reach
        // `set_audio` through the same decode.
        signals.set_audio(audio_frame(&audio));

        let ahead = reading.age + self.output_lag();
        let correction = self
            .lock
            .update(&reading.estimate, ahead, signals.oscillator(), step);
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
        (audio, tempo)
    }

    /// The operator moving the grid an octave: `2.0` for ×2, `0.5` for ÷2.
    ///
    /// **The one decision the estimator cannot make**, and the reason it is a
    /// key rather than a measurement is in `karakuri-audio`'s `tempo` module.
    /// It moves the grid *and* the window together — the window because the
    /// centre is read off the oscillator on the next frame, so tracking
    /// continues in the new octave rather than folding straight back.
    ///
    /// `None` when the new tempo would leave the trackable range, which is the
    /// lock's call rather than this one's.
    pub fn octave(&mut self, signals: &mut Signals, factor: f32) -> Option<Record> {
        let correction = self.lock.octave(factor, signals.oscillator())?;
        let record = tempo_record(&correction);
        apply_tempo(signals, &record);
        // The tracker is told at once rather than waiting for the next frame:
        // an estimate published in between would otherwise be folded into the
        // octave the operator has just left.
        self.input.set_centre_bpm(signals.oscillator().bpm());
        self.last.locked = self.lock.locked();
        self.last.error = self.lock.error();
        Some(record)
    }

    /// A performer tapping the beat. Authoritative, and it goes through the
    /// same record as everything else.
    pub fn tap(&mut self, signals: &mut Signals, at: Instant, since_start: Instant) -> Record {
        let seconds = at.duration_since(since_start).as_secs_f64();
        let correction = self.lock.tap(seconds, self.output_lag(), signals.oscillator());
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

/// **D**, in seconds: the frame queue, at the rate frames are actually
/// arriving, plus the operator's offset for the display's own pipeline.
///
/// A free function because [`Audio`] cannot be constructed without opening a
/// device, and this is the half of the lead that has nothing to do with one:
/// two terms in two different units, which is exactly the arithmetic that is
/// wrong by a factor of a thousand in somebody's first draft and reads as a
/// tuning problem forever after.
pub fn output_lag(frame_interval: f32, display_latency_ms: f32) -> f32 {
    QUEUE_FRAMES * frame_interval + display_latency_ms / 1000.0
}

/// Clamp the operator's offset into [`DISPLAY_LATENCY_RANGE`].
fn clamped_latency(ms: f32) -> f32 {
    ms.clamp(
        *DISPLAY_LATENCY_RANGE.start(),
        *DISPLAY_LATENCY_RANGE.end(),
    )
}

/// A measured frame as the record that carries it.
pub fn audio_record(frame: &AudioFrame) -> Record {
    Record::Audio {
        energy: frame.energy,
        onset: frame.onset,
        bands: frame.bands[..usize::from(frame.band_count).min(MAX_BANDS)].to_vec(),
        confidence: frame.confidence,
    }
}

/// The record as the frame the bus reads, or `None` for a record that is not a
/// measurement at all.
///
/// A record carrying more bands than this build knows names for keeps the
/// bottom [`MAX_BANDS`] of them: the extra ones are names nothing here can ask
/// for, and dropping the *low* bands instead would silently renumber every
/// binding.
///
/// The `None` is the difference between **no measurement** and **a measurement
/// of nothing**, and it is not a hair being split: `None` leaves every name
/// answering exactly as it did before audio existed, invented `energy`
/// included, while a zeroed frame at confidence 0.0 would claim `energy` and
/// answer it with a zero nobody measured.
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

/// Apply a `tempo` record to the session. **The only way a correction reaches
/// the oscillator**, live or on replay.
pub fn apply_tempo(signals: &mut Signals, record: &Record) {
    if let Record::Tempo { bpm, shift, .. } = record {
        signals.correct(*bpm, *shift);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> AudioFrame {
        AudioFrame {
            energy: 0.42,
            onset: 0.75,
            bands: [0.9, 0.4, 0.2, 0.11, 0.05, 0.02, 0.01, 0.0],
            band_count: 8,
            confidence: 1.0,
        }
    }

    /// A decoded frame reproduces the values it was emitted with — the whole
    /// reason the measurement is in the stream at all.
    #[test]
    fn a_frame_survives_the_record_and_the_json_between() {
        let record = audio_record(&frame());
        let line = serde_json::to_string(&record).expect("serialise");
        let decoded: Record = serde_json::from_str(&line).expect("parse");
        assert_eq!(audio_frame(&decoded), Some(frame()));
    }

    /// Silence and absence stay different through the round trip, because that
    /// difference is the one an operator reads off a status line when an
    /// interface dies mid-set.
    #[test]
    fn silence_and_absence_survive_as_different_frames() {
        let silent = AudioFrame::silent(8);
        let absent = AudioFrame::nothing(8);
        assert_eq!(audio_frame(&audio_record(&silent)), Some(silent));
        assert_eq!(audio_frame(&audio_record(&absent)), Some(absent));
        assert_ne!(silent, absent);
    }

    /// A frame measuring fewer bands than the vocabulary has keeps its own
    /// count, so the bands nobody measured stay unanswered rather than becoming
    /// measured zeroes.
    #[test]
    fn a_short_band_list_stays_short() {
        let mut short = frame();
        short.band_count = 3;
        short.bands[3..].fill(0.0);
        let back = audio_frame(&audio_record(&short)).expect("an audio record");
        assert_eq!(back.band_count, 3);
        assert_eq!(back, short);
    }

    /// A record that is not an audio record is **no measurement**, which is a
    /// different thing from a measurement of nothing: it leaves the bus
    /// answering `energy` the way it does with no microphone in the building,
    /// rather than answering it with a zero nobody took.
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
        assert_eq!(measured.provides("energy").expect("claimed").confidence, 0.0);
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

    /// The output lag is the queue plus the offset, and the offset is a dial
    /// with ends.
    ///
    /// The dial's arithmetic is `clamped_latency`'s, called rather than copied:
    /// a test that re-implements the thing it is testing passes whatever the
    /// implementation does.
    #[test]
    fn the_offset_moves_the_output_lag_and_stops_at_its_bounds() {
        let mut latency = clamped_latency(DEFAULT_DISPLAY_LATENCY_MS + DISPLAY_LATENCY_STEP_MS);
        assert_eq!(latency, DEFAULT_DISPLAY_LATENCY_MS + DISPLAY_LATENCY_STEP_MS);
        for _ in 0..100 {
            latency = clamped_latency(latency - DISPLAY_LATENCY_STEP_MS);
        }
        assert_eq!(latency, *DISPLAY_LATENCY_RANGE.start());
        for _ in 0..100 {
            latency = clamped_latency(latency + DISPLAY_LATENCY_STEP_MS);
        }
        assert_eq!(latency, *DISPLAY_LATENCY_RANGE.end());
    }

    /// **D itself**, which nothing checked: two terms in two different units,
    /// summed into seconds. `Audio` cannot be built without a device, so
    /// `output_lag` is a free function and this is the assertion that says the
    /// milliseconds are divided and the frames are not.
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
        let default = output_lag(1.0 / 60.0, DEFAULT_DISPLAY_LATENCY_MS);
        assert!(
            (0.02..0.10).contains(&default),
            "the default output lag is {default} s"
        );
    }
}
