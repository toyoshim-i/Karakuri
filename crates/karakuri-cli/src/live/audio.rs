use super::*;
pub(crate) use karakuri_environment::audio::{apply_tempo, Audio, Grid, LATENCY_OFFSET_STEP_MS};

/// Put the session's grid where the tempo source says the shared grid is.
///
/// The subtraction happens here and not in the source, for the reason
/// `schedule_from` reads a transition's `from` end here: the two numbers — the
/// shared beat and this session's beat — are only both in hand at this moment.
/// A source that sent a shift would be sending a difference from a grid it
/// cannot see.
///
/// That subtraction is the whole of what a shared grid adds. A beat tracker can
/// find how fast beats go and where they are, and cannot find which one is beat
/// one; a number every peer agrees on can, and putting `beats()` onto that
/// number is what makes `bar` the room's bar rather than one counted from
/// whenever this program started.
///
/// How far it is allowed to move is [`tempo_source::Source::correction`]'s, and
/// that bound is not optional: the source is another program, from another
/// repository, released on its own schedule. This used to apply whatever
/// arrived, whole and instantly, which made a helper with a wrong clock able to
/// throw the grid thousands of beats.
///
/// Returns whether the grid is being followed, which is what takes the
/// authority to move it away from the beat tracker.
pub(crate) fn follow_tempo_source(
    source: &mut Option<tempo_source::Source>,
    deck: &mut Deck,
    recorder: &mut Option<session::Recorder>,
) -> audio::Grid {
    let Some(source) = source.as_mut() else {
        return audio::Grid::Owned;
    };
    let heard = source.poll();
    // **Said once and then nothing changes.** The grid is not reset when a
    // source dies: the last tempo it gave is still the best information anyone
    // has, and a show whose beat jumped because a helper crashed would be worse
    // off than one that simply stopped being corrected.
    if let Some(why) = source.unreported_end() {
        eprintln!("tempo source: {why} — the grid holds where it was");
    }
    // A dead source stops being an authority, so the tracker gets the grid back
    // rather than nothing having it.
    if source.ended().is_some() {
        return audio::Grid::Owned;
    }
    let Some(anchor) = heard.anchor else {
        return audio::Grid::Followed;
    };

    let ours = deck.signals().oscillator().beats();
    let (bpm, shift) = match source.correction(anchor, ours, source.now_us()) {
        tempo_source::Correction::Align { bpm, shift } => {
            eprintln!("tempo source: grid aligned, {shift:+.2} beats to {bpm:.1} bpm");
            (bpm, shift)
        }
        tempo_source::Correction::Trim { bpm, shift } => (bpm, shift),
        tempo_source::Correction::Hold => return audio::Grid::Followed,
    };

    let record = karakuri_store::record::Record::Tempo {
        bpm: bpm as f32,
        shift: shift as f32,
        // Not an estimate. A tracker's confidence says how much to believe a
        // guess made from audio; a shared grid is not a guess.
        confidence: 1.0,
    };
    let mut signals = *deck.signals();
    audio::apply_tempo(&mut signals, &record);
    deck.set_signals(signals);
    if let Some(recorder) = recorder.as_mut() {
        recorder.push(record);
    }
    audio::Grid::Followed
}

/// This frame's measurement, and what it does to the session.
///
/// Before the frame is rendered and never inside it. The measured frame and the
/// tempo correction are latched here, exactly where `steps` is measured, so
/// that everything drawn this frame reads one set of values — two bindings
/// sampling `energy` in one frame have to get one answer, or the record saying
/// what this frame saw is a record of neither.
///
/// The session's signals are taken by value, given this frame's measurement,
/// and handed back. `Signals` is `Copy` and the copy carries the phase, so this
/// is not the "restart the session clock" that `Deck::set_signals` warns about
/// — it is the same clock with one frame's input attached.
///
/// A free function rather than a method on `Live`, for the reason [`Clock`] is
/// its own type: it is called from inside the closure that commits a frame,
/// which already holds the deck, so a `&mut self` here would borrow the whole
/// of `Live` a second time. Taking the three pieces it actually touches is also
/// a fair description of what it touches.
pub(crate) fn measure_audio(
    audio: &mut Option<audio::Audio>,
    deck: &mut Deck,
    recorder: &mut Option<session::Recorder>,
    interval: f32,
    steps: u8,
    grid: audio::Grid,
) {
    let Some(audio) = audio.as_mut() else {
        return;
    };
    let mut signals = *deck.signals();
    let (_audio_record, tempo) = audio.frame(&mut signals, interval, f32::from(steps) * DT, grid);
    deck.set_signals(signals);

    // **Swapped, not cloned.** The record carries a `Vec` of bands and this is
    // the frame path; `push_audio` takes this one and leaves an empty shell
    // behind, so the buffer moves and nothing allocates.
    if let Some(recorder) = recorder.as_mut() {
        recorder.push_audio(audio.record_mut());
    }

    // Log non-trim tempo adjustments (e.g. initial acquisition, tap, re-acquisition).
    let reason = audio.reason();
    if let Some(record) = tempo {
        if let (karakuri_store::record::Record::Tempo { bpm, .. }, Some(reason)) = (&record, reason)
        {
            if !matches!(reason, karakuri_audio::Reason::Trim) {
                eprintln!("beat: {reason:?} at {bpm:.1} bpm");
            }
        }
        if let Some(recorder) = recorder.as_mut() {
            recorder.push(record);
        }
    }
}

/// Logs governor pass decisions and parked slots to stderr.
pub(crate) fn report_governing(report: &karakuri_engine::governor::Report, why: &str) {
    eprintln!("{why}: {report}");
    for decision in report.parked() {
        eprintln!(
            "  slot {} parked, request held: {:?}",
            decision.slot, decision.reason
        );
    }
}
