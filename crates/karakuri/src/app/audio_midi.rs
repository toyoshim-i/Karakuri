//! Audio device capture, MIDI controller mapping, audio measurement, and tempo tracking.

use karakuri_console::egui;
use karakuri_console::panel::Panel;
use karakuri_console::view::{
    inspector as inspector_pane, look as look_row, mixer as mixer_bay, tracker_group, AudioIn,
    Tracker, View,
};
use karakuri_engine::{Deck, DeckSlot as EngineSlot};
use karakuri_environment::{audio, midi};
use karakuri_layout::Point;
use karakuri_operation::{BeatSource, GridScale, Operation};
use karakuri_store::record::Record;
use std::time::Instant;
use winit::event_loop::EventLoopProxy;

use crate::bridge::asked_layer;
use crate::gfx::Gfx;

/// Default audio input device identifier to open at startup.
pub(crate) const LISTEN_ON: &str = "default";

/// One simulation step, which is what the audio path has to be told a frame
/// advances the session by so a beat correction lands on the right one.
/// `karakuri-cli` names the same constant for the same reason.
pub(crate) const DT: f32 = karakuri_engine::set::DT;

/// Opens the default audio input stream for room tracking, returning the stream handle and a startup legend line.
///
/// Falls back gracefully with a status message if no audio input device is available (P-0084, P-0094).
pub(crate) fn listening(session_bpm: f32) -> (Option<audio::Audio>, String) {
    match audio::Audio::open(LISTEN_ON, audio::DEFAULT_LATENCY_OFFSET_MS, DT, session_bpm) {
        Ok(open) => {
            let line = format!(
                "audio in: {} at {} Hz — energy, onset and band0..7 are measured from this room \
                 now, and the beat corrects the session's oscillator. output offset {:.0} ms. \
                 the transport row's `audio-in` pill says which input this is and lists the \
                 others; `b` taps the beat, `,` and `.` move the grid an octave, and `o` and \
                 `p` nudge that offset five milliseconds a press.",
                open.description(),
                open.sample_rate(),
                open.latency_offset_ms()
            );
            (Some(open), line)
        }
        Err(why) => (None, unopened(LISTEN_ON, &why)),
    }
}

/// Formats a startup legend message explaining why audio input failed to open.
pub(crate) fn unopened(selector: &str, why: &audio::AudioError) -> String {
    match why {
        audio::AudioError::NoMatch { available, .. } if available.is_empty() => String::from(
            "audio in: none — this machine has no audio inputs, which is a state and not a \
             fault: every signal name answers what it answered before audio existed, and the \
             grid free-runs at the session tempo. the `audio-in` pill says `none` and its card \
             says so too.",
        ),
        audio::AudioError::NoMatch { available, .. } => format!(
            "audio in: `{selector}` is not one of this machine's {} input{} — {}. nothing is \
             open; pick one on the `audio-in` pill.",
            available.len(),
            match available.len() {
                1 => "",
                _ => "s",
            },
            available.join(", ")
        ),
        // Config, Build, SampleFormat: a device that is there and would not
        // start. Said in the audio crate's own words rather than translated —
        // it is the only thing that knows what a host refused.
        other => format!(
            "audio in: none — `{selector}` is there and would not open: {other}. nothing is \
             open; pick another on the `audio-in` pill."
        ),
    }
}

// ---------------------------------------------------------------------------
// The control surface this instrument is playing from
// ---------------------------------------------------------------------------

/// Opens the first available MIDI control surface, returning the surface handle and a startup legend line.
///
/// Falls back to keyboard and pointer controls if no MIDI surface is connected (ADR-0220, ADR-0335).
pub(crate) fn surfaced(
    map: Option<&std::path::Path>,
    waker: EventLoopProxy<()>,
) -> (Option<midi::Surface>, String) {
    match midi::Surface::first(map, move || {
        let _ = waker.send_event(());
    }) {
        Ok((surface, notes)) => {
            let line = surface_line(
                surface.port_name(),
                surface.map_name(),
                map,
                surface.mappings(),
                &notes,
            );
            (Some(surface), line)
        }
        Err(why) => (None, unsurfaced(&why)),
    }
}

/// Formats a startup diagnostic string describing the connected MIDI surface, loaded map, and mapping count.
pub(crate) fn surface_line(
    port: &str,
    map_name: Option<&str>,
    map_path: Option<&std::path::Path>,
    mappings: usize,
    notes: &[String],
) -> String {
    let mut line = format!("midi in: `{port}`");
    match (map_name, map_path) {
        (Some(name), Some(path)) => line.push_str(&format!(
            ", map `{name}` from {} — {mappings} mapping{}.",
            path.display(),
            if mappings == 1 { "" } else { "s" }
        )),
        _ => line.push_str(", no map — turn a knob to map it."),
    }
    for note in notes {
        line.push_str(&format!("\n  midi map: {note}"));
    }
    line
}

/// Returns the path to the store's default learned MIDI mapping file (`<store>/maps/default.map`).
pub(crate) fn learned_map(store: &std::path::Path) -> std::path::PathBuf {
    store
        .join(midi::MAPS)
        .join(format!("{}.{}", midi::DEFAULT_MAP, midi::MAP_SUFFIX))
}

/// Returns the [`Operation`] associated with the control located at point `p`, or `None` if no control was hit.
///
/// Walks the active UI layout (look row, tracker, mixer, and inspector) in press order.
pub(crate) fn asked_at(
    panel: &Panel,
    ctx: &egui::Context,
    view: &View,
    p: Point,
) -> Option<Operation> {
    if let Some(row) = look_row(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
        view.map.as_ref(),
        &view.arrangement,
        view.look,
    ) {
        if let Some(operation) = row.exposure(p) {
            return Some(operation);
        }
    }
    if let Some(group) = tracker_group(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
    ) {
        if let Some(operation) = group.tapped(p) {
            return Some(operation);
        }
    }
    if let Some(bay) = mixer_bay(ctx, panel.layout(), &view.mixer) {
        if let Some(grab) = bay.grab(p) {
            return Some(grab.knob().operation(0.0));
        }
        if let Some(operation) = bay
            .solo(p)
            .or_else(|| bay.mute(p))
            .or_else(|| bay.blend(p))
            .or_else(|| bay.tally(p))
            .or_else(|| bay.mask(p))
        {
            return Some(operation);
        }
    }
    for (index, pane) in view.inspector.iter().enumerate() {
        if let Some(at) = inspector_pane(panel.layout(), index, pane, view.scroll_in(index)) {
            if let Some(grab) = at.grab(pane, p) {
                return Some(grab.knob().operation(0.0));
            }
        }
    }
    None
}

/// Translates a given [`Operation`] into its MIDI map target string representation, or errors if unmappable.
pub(crate) fn target_of(operation: &Operation, deck: &Deck) -> Result<String, String> {
    let named = |what: &str| {
        Err(format!(
            "learn: `{}` is not something a map line can name — {what}. the control's own \
             tooltip says why, on the `⊕ MIDI:` line",
            operation.title()
        ))
    };
    match operation {
        Operation::SetGain { deck, .. } => Ok(format!("gain {deck}")),
        Operation::SetOpacity { deck, .. } => Ok(format!("opacity {deck}")),
        Operation::SetExposure { .. } => Ok("exposure".to_owned()),
        Operation::SetMaskPosition { deck, .. } => Ok(format!("mask-position {deck}")),
        Operation::SetResidency { deck, residency } => {
            Ok(format!("residency {deck} {}", residency.name()))
        }
        Operation::SetBlendMode { deck, blend } => Ok(format!("blend {deck} {}", blend.name())),
        Operation::TapBeat => Ok("tap".to_owned()),
        Operation::WriteParam {
            deck: slot, param, ..
        } => {
            let index = usize::from(*slot);
            if index >= deck.slot_count() {
                return Err(karakuri_environment::no_such_slot(index, deck.slot_count()));
            }
            let set = deck.slot(EngineSlot(*slot)).set();
            let at = set.published().iter().position(|control| {
                control.key == param.key
                    && control
                        .at
                        .map(|(kind, index)| karakuri_operation::NodeAddress {
                            layer: asked_layer(kind),
                            index,
                        })
                        == param.node
            });
            match at {
                Some(at) => Ok(format!("param {slot} {}", at + 1)),
                // Unmapped if the parameter is not exposed on the published interface.
                None => named(
                    "that deck's Set does not publish it, so it has no position to count \
                     to — put it on the interface and learn it again",
                ),
            }
        }
        // Mask shape operations cannot be bound directly to a MIDI CC because they carry an orientation angle (ADR-0202).
        Operation::SetMaskShape { .. } => {
            named("a map line has no way to write an angle, so the shape has no spelling")
        }
        _ => named("no map target names it"),
    }
}

/// Formats a diagnostic message explaining why no MIDI control surface could be initialized.
pub(crate) fn unsurfaced(why: &str) -> String {
    if why.contains("there are no MIDI inputs") {
        return String::from(
            "midi in: none — nothing is plugged in, which is a state and not a fault: every              control on this panel is reached by the pointer and by the keyboard, and that is              every run this program has had. plug a surface in and start again.",
        );
    }
    format!(
        "midi in: none — {why}. the panel runs; every control is still under the pointer and          under a key. this port is opened once, at startup, so a surface freed later is          reached by starting again."
    )
}

/// Derives the current [`AudioIn`] UI state from the active audio session.
pub(crate) fn told(open: Option<&audio::Audio>) -> AudioIn {
    let mut told = AudioIn::NONE;
    told.device = open.map(|open| open.description().to_owned());
    told
}

/// Handles an [`Operation::AttachBeatSource`] request, opening the selected audio input and updating the UI state.
pub(crate) fn attached(
    open: &mut Option<audio::Audio>,
    session_bpm: f32,
    told_pill: &mut Option<AudioIn>,
    operation: &Operation,
) -> Option<String> {
    let Operation::AttachBeatSource { source } = operation else {
        return None;
    };
    let selector = match source {
        BeatSource::AudioInput(selector) => selector,
        BeatSource::Process(command) => {
            return Some(format!(
                "  attach: `{command}` is a process, and nothing on this panel starts one — \
                 `karakuri-cli --tempo-source` is where that half of the row lives"
            ))
        }
    };
    let offset = open
        .as_ref()
        .map(|open| open.latency_offset_ms())
        .unwrap_or(audio::DEFAULT_LATENCY_OFFSET_MS);
    match audio::Audio::open(selector, offset, DT, session_bpm) {
        Ok(opened) => {
            let line = format!(
                "  attach: {} at {} Hz -> AttachBeatSource -> no record, and that is settled: \
                 nothing in the session stream says what the beat was taken from. the grid \
                 follows this room now, at offset {:.0} ms",
                opened.description(),
                opened.sample_rate(),
                opened.latency_offset_ms()
            );
            *open = Some(opened);
            *told_pill = Some(told(open.as_ref()));
            Some(line)
        }
        Err(why) => Some(format!(
            "  attach: {} — {}",
            selector,
            match open.as_ref() {
                Some(open) => format!("{why}. `{}` is still open", open.description()),
                None => format!("{why}. nothing is open"),
            }
        )),
    }
}

/// Diagnostic message returned when attempting to adjust latency offset with no audio input active.
pub(crate) const NO_ROOM_FOR_AN_OFFSET: &str =
    "offset: no audio input — the offset is the delay between \
                                     what a room hears and what it sees, and there is no room. \
                                     open one on the transport row's `audio-in` pill";

/// Formats an informational readout string describing the applied audio latency offset.
pub(crate) fn offset_said(asked: f32, now: f32) -> String {
    let sense = match now < 0.0 {
        true => "the picture waits for the music",
        false => "the picture leads the music",
    };
    let held = match (asked - now).abs() > f32::EPSILON {
        true => format!(
            " — held at {:+.0} ms, which is as far either way as it goes",
            now
        ),
        false => String::new(),
    };
    format!("  offset: {now:+.0} ms — {sense}{held}")
}

/// Adjusts the audio latency offset for the active session, clamping within supported boundaries.
pub(crate) fn nudged(open: &mut Option<audio::Audio>, operation: &Operation) -> Option<String> {
    let Operation::SetLatencyOffset { ms } = *operation else {
        return None;
    };
    let Some(open) = open.as_mut() else {
        return Some(format!("  {NO_ROOM_FOR_AN_OFFSET}"));
    };
    let now = open.nudge_latency_offset(ms - open.latency_offset_ms());
    Some(offset_said(ms, now))
}

/// Updates the audio beat tracking window to target a new manual free-run BPM setting.
pub(crate) fn retargeted(open: &mut Option<audio::Audio>, operation: &Operation) -> Option<String> {
    let Operation::SetFreeRunTempo { bpm } = *operation else {
        return None;
    };
    let open = open.as_mut()?;
    open.set_tempo(bpm);
    Some(format!(
        "  tempo: the room is still being tracked — the window moved to {bpm:.1} with the grid, \
         and the next estimate is made around it rather than about where the grid was"
    ))
}

/// Processes one audio frame: updates signal metrics, estimates tempo, and records audio stream data.
pub(crate) fn measure_audio(
    open: &mut Option<audio::Audio>,
    deck: &mut Deck,
    interval: f32,
    steps: u8,
    recorder: Option<&mut karakuri_environment::session::Recorder>,
) {
    let Some(open) = open.as_mut() else {
        return;
    };
    let mut signals = *deck.signals();
    let (_audio, tempo) = open.frame(
        &mut signals,
        interval,
        f32::from(steps) * DT,
        audio::Grid::Owned,
    );
    deck.set_signals(signals);

    // Append audio frame metrics to the session recorder if recording is active.
    let recorder = match recorder {
        Some(recorder) => {
            recorder.push_audio(open.record_mut());
            Some(recorder)
        }
        None => None,
    };

    // Log beat tracker state changes, ignoring recurring sub-frame trims (P-0094).
    let reason = open.reason();
    if let (Some(Record::Tempo { bpm, .. }), Some(reason)) = (&tempo, reason) {
        if !matches!(reason, karakuri_environment::audio::Reason::Trim) {
            println!("beat: {reason:?} at {bpm:.1} bpm");
        }
    }
    if let (Some(recorder), Some(record)) = (recorder, tempo) {
        recorder.push(record);
    }
}

/// Returns the current [`Tracker`] state for UI display based on active audio session and session BPM.
pub(crate) fn tracking(open: Option<&audio::Audio>, bpm: f32) -> Tracker {
    Tracker {
        offset_ms: open.map(audio::Audio::latency_offset_ms),
        halve: audio::BPM_RANGE.contains(&(bpm * 0.5)),
        double: audio::BPM_RANGE.contains(&(bpm * 2.0)),
    }
}

/// The two operations that move the room's tracker, performed against the
/// session this program opened — and `None` for every operation that is not one
/// of them.
pub(crate) fn tracked(gfx: &mut Gfx, started: Instant, operation: &Operation) -> Option<String> {
    match operation {
        Operation::TapBeat => Some(tapped(
            &mut gfx.audio,
            &mut gfx.engine.deck,
            Instant::now(),
            started,
        )),
        Operation::ScaleGrid { by } => Some(scaled(&mut gfx.audio, &mut gfx.engine.deck, *by)),
        _ => None,
    }
}

/// Applies a beat tap event against the audio tracker and deck oscillator, returning a summary message.
pub(crate) fn tapped(
    open: &mut Option<audio::Audio>,
    deck: &mut Deck,
    at: Instant,
    started: Instant,
) -> String {
    let Some(open) = open.as_mut() else {
        return String::from(
            "tap: no audio input — a tap sets the grid this room is being tracked against, and \
             there is no room. open one on the transport row's `audio-in` pill",
        );
    };
    let mut signals = *deck.signals();
    let record = open.tap(&mut signals, at, started);
    deck.set_signals(signals);
    match record {
        Record::Tempo { bpm, shift, .. } => format!(
            "tap: -> Record::Tempo {{ bpm: {bpm:.1}, shift: {shift:+.3} }} — three taps or more \
              set the tempo and any tap sets the phase"
        ),
        other => format!("tap: -> {other:?}"),
    }
}

/// Scales the beat tracker grid by an octave (halve or double), clamping to the supported BPM range.
pub(crate) fn scaled(open: &mut Option<audio::Audio>, deck: &mut Deck, by: GridScale) -> String {
    let (factor, word) = match by {
        GridScale::Halve => (0.5, "half"),
        GridScale::Double => (2.0, "double"),
    };
    let Some(open) = open.as_mut() else {
        return format!(
            "grid: no audio input — {word} moves the tracker's octave window, which only exists \
             while a room is being tracked. open one on the transport row's `audio-in` pill"
        );
    };
    let mut signals = *deck.signals();
    let moved = open.octave(&mut signals, factor);
    deck.set_signals(signals);
    match moved {
        Some(Record::Tempo { bpm, .. }) => format!(
            "grid: {word} -> Record::Tempo {{ bpm: {bpm:.1} }} — the tracker's window went with \
             it, and the phase did not move"
        ),
        Some(other) => format!("grid: {word} -> {other:?}"),
        None => format!(
            "grid: {word} refused — the result would leave the trackable range, and the next \
             estimate that disagreed would drag the grid straight back"
        ),
    }
}
