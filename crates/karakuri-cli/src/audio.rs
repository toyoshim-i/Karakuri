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
//! **Both halves this module was waiting on are built.** `--record-session`
//! writes these records to disk and `--replay` reads them back instead of
//! opening a device, so a binding to `energy` replays against what the room
//! actually sounded like rather than against the bus's invented values.

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

/// A starting value for everything past the two outputs that **cannot be
/// measured here**. 20 ms is a plausible display pipeline and nothing more: it
/// is where the operator starts adjusting, not an answer.
///
/// See `karakuri-audio`'s "Why the offset is the answer and not a better
/// measurement". A sound path and a picture path leave this machine separately
/// and neither ends at it, so the only instrument that can read this is a
/// person standing where the audience stands.
pub const DEFAULT_LATENCY_OFFSET_MS: f32 = 20.0;

/// One press of the offset keys.
pub const LATENCY_OFFSET_STEP_MS: f32 = 5.0;

/// The bounds the offset is held inside. **Signed, and that is not symmetry for
/// its own sake**: which of the two outputs is the late one depends on the
/// room. A PA delayed to the back of a hall, or a desk with processing on the
/// master, puts the sound behind a projector that had looked slow; the
/// correction then has to lead *less*, and a floor at zero would leave the
/// operator holding a control that cannot reach the answer.
///
/// 200 ms either way is past any single device and well into the region where a
/// performer has mistaken a whole beat for an offset.
pub const LATENCY_OFFSET_RANGE: std::ops::RangeInclusive<f32> = -200.0..=200.0;

/// The audio session: an open input, the beat lock, and the operator's offset.
pub struct Audio {
    input: AudioInput,
    lock: BeatLock,
    latency_offset_ms: f32,
    /// A smoothed frame interval, for the queue part of the output lag. Fed
    /// from the same elapsed-time measurement `steps` comes from — the frame
    /// rate is a property of the machine and the display, and guessing it from
    /// `dt` would be wrong on exactly the 120 Hz panel this was built on.
    frame_interval: f32,
    /// What was last read, for the status line.
    last: Status,
    /// **This frame's `Record::Audio`, reused.** Rewritten in place every frame
    /// and handed out by reference.
    ///
    /// The record carries its bands as a `Vec` on purpose — the length is the
    /// band count, so a stream with more bands than a reader knows about still
    /// decodes, and `record.rs` argues that at length. Building a fresh one per
    /// frame would put a heap allocation on the render thread, which
    /// `README.md` forbids without a size qualifier and deliberately: "one
    /// small allocation" is the argument that ends with a hitch nobody can
    /// account for. `Vec::clear` keeps the buffer, so the only allocation is
    /// the one here, before the first frame.
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
    /// `session_bpm` is `--bpm`, and it does **two** jobs: it is what the
    /// oscillator free-runs at, and it is where the tempo tracker's octave
    /// window starts. They are the same number because they are the same
    /// statement — "this is roughly the tempo" — and after the first lock the
    /// window centre simply follows the grid. See `karakuri-audio`'s `tempo`
    /// module.
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

    /// **D**: the frame queue, plus the operator's offset for everything past
    /// the two outputs. See `karakuri-audio`'s crate doc.
    pub fn output_lag(&self) -> f32 {
        output_lag(self.frame_interval, self.latency_offset_ms)
    }

    /// One frame's worth of audio: read the device, emit the records, and apply
    /// what they say to the session's signals.
    ///
    /// Returns the records, which is what a session writer would take. Nothing
    /// writes them yet — see the module doc — but they are *built here and read
    /// back here*, so the path the engine is driven through is the record's
    /// rather than one that happens to agree with it.
    ///
    /// The audio record is handed out **by reference and is overwritten next
    /// frame**: it is one buffer, reused, because building a fresh one would
    /// allocate on the render thread. A writer serialises it before returning;
    /// a caller that wants to keep it past the frame owes itself a clone.
    ///
    /// Never blocks: the device read is a `try_lock` that keeps the previous
    /// value on contention, and nothing on this path allocates.
    pub fn frame(
        &mut self,
        signals: &mut Signals,
        elapsed: f32,
        step: f32,
    ) -> (&Record, Option<Record>) {
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
        write_audio_record(&mut self.audio, &reading.frame);
        // Read back rather than used directly, which is the whole point: the
        // live path and a replay reach `set_audio` through the same decode, so
        // the conversion is exercised every frame instead of only by a test.
        signals.set_audio(audio_frame(&self.audio));

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
        (&self.audio, tempo)
    }

    /// **This frame's audio record, to be taken.**
    ///
    /// Handed out mutably so a session recorder can *swap* it for an empty
    /// shell rather than clone it: the record carries a `Vec` of bands and this
    /// is the frame path. See `session::Recorder::push_audio`.
    ///
    /// Whatever is left here is overwritten by the next [`Audio::frame`], so a
    /// caller that swaps in a shell loses nothing.
    pub fn record_mut(&mut self) -> &mut Record {
        &mut self.audio
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
/// arriving, plus the operator's offset for everything past the two outputs.
///
/// **May be negative**, when the offset is turned down past the queue — a room
/// whose sound arrives later than its picture. Nothing downstream needs it
/// positive: the lead it feeds is a signed quantity all the way into
/// `Oscillator::correct`, and a negative one is a correction that trails the
/// measurement instead of leading it, which is exactly what such a room wants.
///
/// A free function because [`Audio`] cannot be constructed without opening a
/// device, and this is the half of the lead that has nothing to do with one:
/// two terms in two different units, which is exactly the arithmetic that is
/// wrong by a factor of a thousand in somebody's first draft and reads as a
/// tuning problem forever after.
pub fn output_lag(frame_interval: f32, latency_offset_ms: f32) -> f32 {
    QUEUE_FRAMES * frame_interval + latency_offset_ms / 1000.0
}

/// Clamp the operator's offset into [`LATENCY_OFFSET_RANGE`].
fn clamped_latency(ms: f32) -> f32 {
    ms.clamp(
        *LATENCY_OFFSET_RANGE.start(),
        *LATENCY_OFFSET_RANGE.end(),
    )
}

/// A measured frame into an existing [`Record::Audio`], **reusing its band
/// buffer**. `clear` keeps the allocation, so this allocates nothing once the
/// buffer has been sized once — which is what lets the frame path emit a record
/// at all.
///
/// Panics on anything but a `Record::Audio`, deliberately: the caller owns the
/// buffer it is passing and cannot be handed the wrong variant by accident.
/// Silently doing nothing would leave the previous frame's measurement in
/// place and present it as this one's.
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

    /// A record built from scratch. **Tests only, and the allocation is why**:
    /// the frame path rewrites one record in place, so a function that returns
    /// a fresh one has no caller there and would be a standing invitation to
    /// become one.
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

    /// **Rewriting the record does not touch the heap.** `Audio::frame` emits
    /// one of these per frame on the render thread, where `README.md` allows no
    /// allocation at all, so the band buffer has to be the one from before.
    ///
    /// A counting allocator would be the direct assertion, but a
    /// `#[global_allocator]` is per binary and this is one — it would count
    /// every other test in the crate. The buffer's **pointer and capacity**
    /// are the observable consequence instead, and they are not a proxy: a
    /// `to_vec`, a fresh `Vec`, or any growth past the reserved length moves
    /// one or both. Band counts are varied across the calls, including up to
    /// the maximum and back down, because a buffer that is only ever written at
    /// one length would hold under an implementation that reallocates on any
    /// change of length.
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
        let default = output_lag(1.0 / 60.0, DEFAULT_LATENCY_OFFSET_MS);
        assert!(
            (0.02..0.10).contains(&default),
            "the default output lag is {default} s"
        );
    }

    /// **The offset reaches below zero and takes the lead with it.**
    ///
    /// A room whose sound arrives after its picture — a delayed PA, processing
    /// on the master — needs the correction to lead *less* than the queue
    /// alone, and past a point to trail it. A floor at zero would hand the
    /// operator a control that stops short of the answer, and there is no other
    /// control: nothing at this end can see either output path.
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
