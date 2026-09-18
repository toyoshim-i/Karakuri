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

/// And it opens something rather than nothing, which is the choice that
/// matters for what this instrument is: material that moves with the room is
/// what the panel looks like, and an instrument that listens only after being
/// asked comes up looking like one that cannot. `default` is what a machine
/// answers when nobody has chosen, which is exactly the state a program that
/// has just started is in.
pub(crate) const LISTEN_ON: &str = "default";

/// One simulation step, which is what the audio path has to be told a frame
/// advances the session by so a beat correction lands on the right one.
/// `karakuri-cli` names the same constant for the same reason.
pub(crate) const DT: f32 = karakuri_engine::set::DT;

/// Open the input this program listens on, and say what happened.
///
/// Returns the session and one line for the legend — never a refusal that stops
/// the run, and that is the decision rather than an omission. The three cases
/// it has to be right about are the three the window can meet, and two
/// principles point in different directions across them:
///
/// 1. No device at all.
/// [P-0084](../../../docs/principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md)
/// — *a quiet room is not a missing microphone* — and neither is a missing
/// microphone a fault. Nobody asked for one here: this program opens the
/// default because that is what an instrument does, and a machine with no input
/// is a machine where every name goes on answering what it answered before
/// audio existed and the oscillator free-runs. It is said out loud, once, and
/// the run continues. Exiting would mean a laptop with its microphone switched
/// off cannot open the panel at all. 2. A device that was named and is not
/// there. A different case, and
/// [P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
/// is why: somebody said *that one*, and going quietly on with a different one
/// — or with none — is the silently wrong picture. It cannot happen *here*,
/// because nothing names an input at launch; it happens at the pill, where the
/// list an operator picked from was read at the press and a device can have
/// gone away since. [`attached`] is that case and it is loud there. Loud and
/// not fatal, which is where this program parts from `karakuri-cli`: a window
/// with a set on it must not close because an interface was unplugged, and the
/// operator is standing in front of the refusal. 3. A device that goes away
/// mid-set. Nothing here notices, deliberately, and that *is* the answer:
/// `karakuri-audio`'s `staleness` takes the confidence of both the signals and
/// the tempo estimate to zero over half a second, every bound parameter is
/// handed back to the value it had, and the grid free-runs from wherever it
/// was. A watchdog that re-opened the stream would be a second answer to a
/// question that already has one, and it would re-lock the grid to a room in
/// the middle of a set. What an operator does about it is pick again on the
/// pill.
///
/// A free function rather than a step of `resumed`, for the reason
/// [`sources_from`] is one: `resumed` cannot be called from a test, and a
/// refusal nobody can reach is a refusal nobody checked. See [`unopened`],
/// which is the half of it that has no device in it at all.
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

/// What to say about an input that did not open, and which of the two kinds of
/// nothing it was.
///
/// Split out from [`listening`] because it is the whole of the judgement and
/// none of the device: a machine with no inputs and a machine whose default
/// vanished are two sentences, and the difference between them is the
/// difference between P-0084 and P-0094. Being a function of an error and a
/// string, it is checkable where no input can be opened at all — which is every
/// machine a test runs on, whatever it happens to have plugged in.
///
/// The empty case is not apologetic and the non-empty one is not calm. A
/// machine with no inputs is a state; a machine with inputs where the one asked
/// for is not among them is somebody's mistake or somebody's cable, and the
/// list is what they need rather than an invitation to go and look.
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

/// Open the surface this program plays from, and say what happened.
///
/// Returns the surface and one line for the legend — never a refusal that stops
/// the run, which is [`listening`]'s decision one door along and it is the same
/// decision for the same reason. Three cases, and the three are not the
/// microphone's three:
///
/// 1. Nothing plugged in, which is most machines and is a state rather than a
/// fault. Nobody named a port: this program takes whatever is there because
/// that is what an instrument does, and a run with no surface is a run played
/// with the pointer and the keyboard, which is every run this program has had
/// until now. Said out loud, once. 2. A port that is there and will not open —
/// taken by another program, usually. Said in the port's own words, and the run
/// continues: a window with a set on it must not fail to start because
/// something else has the controller. 3. A port that goes away mid-set. Nothing
/// here notices, deliberately. `midir` holds the connection and a device
/// unplugged stops sending; every control on this panel is still under the
/// pointer and under a key, and nothing on the deck moves on its own. What an
/// operator does about it is plug it back in and restart, which is what the
/// legend says — there is no pill to re-open one, because the transport row's
/// `map` is one of the two controls the mock draws and this console does not.
///
/// Which port is the first one there is, and it is not a flag. `karakuri-cli`
/// is told with `--midi-in` and refuses the run without the one it was told;
/// [`USAGE`] declines that flag by name for
/// [ADR-0220](../../../docs/adr/0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md)'s
/// reason read one column along — this program is the instrument, and a
/// launch-time answer is one an operator standing in front of the panel cannot
/// change. So it opens what is there and says which, exactly as `audio-in`
/// does.
///
/// Which map is [`midi::map_for`] — the operator's own under the store, then
/// the one that ships — and the path is printed rather than described, for the
/// reason every line of [`Readout::print_legend`] is derived.
///
/// A free function rather than a step of `resumed`, for [`listening`]'s reason:
/// `resumed` cannot be called from a test, and the sentences are the half of
/// this that has no device in it. See [`surface_line`] and [`unsurfaced`],
/// which are that half.
pub(crate) fn surfaced(
    map: Option<&std::path::Path>,
    waker: EventLoopProxy<()>,
) -> (Option<midi::Surface>, String) {
    // **The wake, and it is the whole of what this closure is.** `send_event`
    // is called on the MIDI thread once per message; `()` says *ask again* and
    // [`App::user_event`] is what asks. The error is dropped because it means
    // the loop has gone, which means the run is ending.
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

/// What the legend says about an open surface, and it is a function of four
/// facts and nothing else — so it is checkable on a machine with no MIDI on it
/// at all, which is every machine a test runs on here.
///
/// The map's path is printed beside its name because the name alone cannot say
/// which of the two tiers answered: `default` under the store and `surface` in
/// the preset library are two different files and an operator who has just
/// learned one wants to know which of them is loaded.
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
            ", map `{name}` from {} — {mappings} mapping{}",
            path.display(),
            if mappings == 1 { "" } else { "s" }
        )),
        // **No map is a state and the sentence is the one the operator needs
        // next**, which is `karakuri-environment`'s own words for it: a
        // surface with no map still reports what it sends.
        _ => line.push_str(
            ", no map — turn a knob and this will print the line that would map it, once per              control",
        ),
    }
    line.push_str(
        ". every mapped message becomes the operation the map names and is performed on the          frame it arrives on, where a press on the mixer's fader is performed — so a knob and          a hand write one record and a session recorded from this surface replays with          neither the surface nor the map attached. nothing on this panel names the map: the          transport row's `map` pill is one of the two controls the mock draws and this          console does not.",
    );
    for note in notes {
        line.push_str(&format!("\n  midi map: {note}"));
    }
    line
}

/// Where a learned map is written — `<store>/maps/default.map`, the first of
/// `karakuri_environment::midi::map_for`'s two tiers.
///
/// It is always this file, whatever map the run loaded: a run playing the
/// shipped `examples/surface.map` and learning a control writes into the store,
/// which is
/// [P-0096](../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)
/// held rather than argued. `default` is the name a program with no way to ask
/// uses; naming another is the `map` pill, which is a readout.
pub(crate) fn learned_map(store: &std::path::Path) -> std::path::PathBuf {
    store
        .join(midi::MAPS)
        .join(format!("{}.{}", midi::DEFAULT_MAP, midi::MAP_SUFFIX))
}

/// What a press on the control under the pointer would ask for, or `None` where
/// the pointer is on no control at all.
///
/// This is learn's half of the pointer question, and it is deliberately the
/// *press* derivation rather than a new one: a control's identity is what a
/// press on it asks the deck for, so a knob learned against it moves exactly
/// what a click moves. The hover layer answers *which* control
/// ([`karakuri_console::hover::Hover::resting`]) and this answers *what it is*,
/// and the two walk the same `view::` derivations — a second geometry here
/// would be a second answer that could disagree with the tip the operator is
/// reading while they learn.
///
/// The value is a placeholder and is thrown away. What learn wants is the
/// *address*, which is an operation with its value elided — `LaneTarget`'s own
/// sentence one route along (ADR-0321). `Knob::operation` is the only way to
/// get one out of the console, so this asks it at zero and [`target_of`] reads
/// past the value.
///
/// The order is the hover layer's, which is the press order: the controls
/// inside a container before the container. It matters in the Mixer, where a
/// strip's chips sit inside the strip.
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

/// The right-hand side of the map line that reaches this control, or the
/// sentence saying why there is none.
///
/// # It is `karakuri_midi`'s own list, read the other way
///
/// The eight arms are exactly `Target::spelled`'s eight, which is what makes
/// this safe: every string it returns is a string that crate's parser accepts,
/// and `a_learned_target_is_one_the_grammar_accepts` is what holds that. A
/// ninth spelling invented here would be a line an operator's file could not
/// hold.
///
/// # The parameter arm is the whole reason this function exists
///
/// Every other operation carries its own address — a slot number, a word from a
/// closed list. `WriteParam` carries a name, and a map line holds a position
/// (ADR-0268): *knob 3 is knob 3 whatever Set is loaded*, and binding to the
/// name would make the mapping a cost paid again on every swap. So this is
/// where the name goes back to being a position, against the Set that is in the
/// deck — `Set::published()` in order, counting from one, which is the number
/// the Inspector draws beside the row and the same reading
/// `karakuri_environment::midi::Decks` makes in the other direction.
///
/// # A refusal is a sentence and not a silence
///
/// A control a map line cannot name is most of this panel, and the operator
/// pointing at one is owed the reason —
/// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md).
/// The reasons are the console page's, one control at a time, and this points
/// at the tooltip rather than repeating forty of them.
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
                // **A control off the published interface**, which the pane
                // draws with its number and its fader gone (`view::Param::ord`
                // is `None` for one). A position is exactly what a knob
                // counts, so there is nothing to bind — and the answer is to
                // publish it, which is a press in the same pane.
                None => named(
                    "that deck's Set does not publish it, so it has no position to count \
                     to — put it on the interface and learn it again",
                ),
            }
        }
        // **A mask's shape is the sharp one and it is not an oversight.** A
        // map line can say a slot, a word from a closed list or a range, and
        // this operation carries an angle as well as a kind — so a pad naming
        // the kind would have to invent the angle beside it (ADR-0202,
        // ADR-0209).
        Operation::SetMaskShape { .. } => {
            named("a map line has no way to write an angle, so the shape has no spelling")
        }
        _ => named("no map target names it"),
    }
}

/// What to say about a surface that did not open, and which of the two kinds of
/// nothing it was — [`unopened`]'s shape one door along, and split out for its
/// reason.
///
/// The empty case is not apologetic. A machine with nothing plugged in is the
/// ordinary state of this program and always has been; saying it as a failure
/// would read as one. A machine that *has* inputs and would not open the first
/// is somebody's other program holding the port, and the message
/// `karakuri-midi` came back with is the only thing that knows which.
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

/// What the `audio-in` pill reads, out of the session this program opened.
///
/// One line, and it is a function rather than an assignment for the reason
/// [`transport`] is one: it is the seam, and there is exactly one place the
/// answer is derived. The card's list is not here — it is read on the press
/// that opens the card and nowhere else (P-0091), so a reading taken every
/// frame would be a directory read on the frame path with a microphone in place
/// of the directory.
pub(crate) fn told(open: Option<&audio::Audio>) -> AudioIn {
    let mut told = AudioIn::NONE;
    told.device = open.map(|open| open.description().to_owned());
    told
}

/// A press on one of the `audio-in` card's rows, performed, and what this file
/// says about it. `None` for every operation that is not it, exactly as
/// [`arrangement`] and [`pointed`] answer `None` for everything that is not
/// theirs.
///
/// This is the second of [`listening`]'s three cases and the only one that can
/// arrive during a set: the card lists what the host had at the press that
/// opened it, and an interface unplugged between that press and this one is a
/// name the operator picked that is not there any more. P-0094 — the refusal is
/// printed with the list as it is *now*, and the input that was already open
/// stays open: dropping it would answer a mistyped pick by taking away the
/// room, which is the one thing nobody asked for.
///
/// `AttachBeatSource` writes no record (`written` answers `Silent(NoRecord)`:
/// no session-stream variant carries what the beat is taken from), so nothing
/// downstream of this moves the deck. What moves is this program's own audio
/// session and the pill that reads it.
///
/// A `BeatSource::Process` reaches here and is declined in one sentence: the
/// panel has no control that names one and `--tempo-source` is
/// `karakuri-cli`'s. It is answered rather than ignored, because an operation
/// that arrives and does nothing at all is the failure P-0094 is about.
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
    // The offset the operator has already dialled in survives the change of
    // device: it is a property of this room's outputs and not of its input,
    // which is the whole of what `LATENCY_OFFSET_RANGE`'s documentation is
    // about. A new session at the default would silently undo it.
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
        // **The one that was open stays open**, and the pill goes on naming
        // it: what failed is the pick, not the room.
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

/// What the offset keys say on a panel with no input attached.
///
/// `docs/manual/console.html` is the specification and it is plain about it:
/// *"It only means anything with an audio input attached, and the audio-in pill
/// is what says whether there is one."* So the press changes nothing, says why,
/// and names the control that would fix it — [`tapped`]'s and [`scaled`]'s
/// sentence for the same state, one row along.
pub(crate) const NO_ROOM_FOR_AN_OFFSET: &str =
    "offset: no audio input — the offset is the delay between \
                                     what a room hears and what it sees, and there is no room. \
                                     open one on the transport row's `audio-in` pill";

/// What to say about an offset that moved, out of what was asked for and
/// what the session came back with.
///
/// A function of two numbers and nothing else, so that both halves of
/// `console.html`'s contract are checkable without a device:
///
/// - The sign, in words. *"Negative and the picture waits for the music,
///   positive and it leads"* — the page says it in words rather than leaving
///   `−15 ms` to be interpreted, and so does this.
/// - The bound, when it bit. The value is *"held inside 200 ms either
///   way"*, which `karakuri_environment::audio` enforces and this reports: a
///   press that asked for 205 and got 200 is a control at the end of its
///   travel, and a control that answers the same number twice with nothing
///   said is indistinguishable from a broken one (P-0094).
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

/// The latency offset, performed against the session this program opened, and
/// `None` for every operation that is not it — [`attached`]'s shape, one
/// control along, and beside it in [`App::performed`] for the same reason.
///
/// `SetLatencyOffset` writes no record (`written` answers `Silent(NoRecord)`:
/// nothing in the session stream carries a delay between two outputs, which is
/// a property of a room and not of a performance), so nothing downstream of
/// this moves the deck. What moves is the lead every beat correction is applied
/// with — `Audio::output_lag` — and the frame the picture is drawn on relative
/// to it.
///
/// The operation is absolute and this is where it lands. It is applied through
/// `Audio::nudge_latency_offset`, which is the only way in and is the one that
/// clamps: the offset is held inside `LATENCY_OFFSET_RANGE` there, so this file
/// states no bound of its own and cannot state a different one. A *setting*
/// becomes the step that reaches it, which is what lets a fader emit this
/// operation the day one exists without a second application path.
///
/// With nothing open there is nothing to offset, and the key arm says so before
/// an operation is built — see [`NO_ROOM_FOR_AN_OFFSET`]. This arm answers the
/// case an operation arrives from anywhere else in that state, because an
/// operation that arrives and does nothing at all is the failure P-0094 is
/// about.
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

/// The grid's tempo, named by hand and handed to the room's tracker — and
/// `None` for every operation that is not [`Operation::SetFreeRunTempo`].
///
/// # It does not move the grid, and that is what separates it from [`nudged`]
///
/// The offset above is `Silent(NoRecord)`: nothing in a session stream carries
/// a delay between two outputs, so the session this program opened is the only
/// thing that holds it. A free-run tempo is the opposite — `written` answers a
/// `Record::Tempo` for it, and [`apply`] is what applies it, through the same
/// `audio::apply_tempo` a replay goes through. Moving the oscillator here as
/// well would be a second route into the engine, taken only when a device
/// happens to be open (P-0090).
///
/// So what this hands over is the state the record does not carry: the beat
/// lock's run of evidence, and the window the tracker searches. Both are
/// `Audio::set_tempo`, and the argument for each is there and in
/// `karakuri_audio`'s `BeatLock::retarget`.
///
/// # With nothing open it says nothing, and that is the state it is for
///
/// [`nudged`] refuses out loud with no session — an offset belongs to one, and
/// an operation that arrives and does nothing at all is what P-0094 is about.
/// This is the other way round: *what the grid runs at with nothing driving it*
/// is exactly the case with no device, [`apply`] moves the oscillator and
/// prints the line, and a refusal here would be this file talking about a
/// tracker that is not part of the operation. What it says when there is one is
/// that the set was accepted and the room is still being tracked, which is the
/// one thing an operator cannot see from the tempo alone.
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

/// One frame's worth of audio: read the room, and hand the session what it
/// said.
///
/// The same three lines `karakuri-cli`'s `measure_audio` is, minus the two
/// halves this program does not have — there is no session recorder to hand the
/// record to, and no tempo source to yield the grid to, so the grid is always
/// this tracker's ([`audio::Grid::Owned`]).
///
/// The signals are copied out of the deck and back in, which is what
/// `Deck::signals` and `set_signals` are for: the bus is a `Copy` value and the
/// deck is the model of record for it, so an `AudioFrame` reaching a binding
/// goes through the deck rather than round it.
///
/// `interval` is how fast frames are actually arriving, which is half the
/// output lag a beat correction leads by, and `steps` is how much session this
/// frame is worth. Both are [`App::clock`]'s one measurement, which is
/// `karakuri-cli`'s arrangement of the same call: the interval the step count
/// was derived from is the interval the lag is built from, so the two cannot
/// disagree about how long this frame was.
///
/// It is not the transport row's rate inverted: [`Costs::rate`] is an average
/// over half a second, and the lag uses the interval between this frame and the
/// last one, which is the number the clock takes for the `tick`.
/// `Audio::frame` still ignores an interval outside `(0, 1)`: the smoothed
/// value holds, which is the right answer for the first frame of a run and for
/// one that followed a stall.
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

    // **Into the session, where one is being recorded**, and this is the half
    // this program did not have when the paragraph above was written.
    //
    // **Swapped, not cloned**, which is `karakuri-cli`'s own line: the record
    // carries a `Vec` of bands and this is the frame path, so `push_audio`
    // takes this one and leaves an empty shell behind. Nothing allocates. A
    // frame with no shell free is counted rather than dropped silently — see
    // `session::Recorder::push_audio`.
    //
    // **The measurement and the correction are both pushed, in that order**,
    // because that is the order they happened in: a replay reading the stream
    // applies the tempo the frame decided after the audio the frame heard.
    //
    // **After the sentence below rather than before it**, which costs nothing
    // and keeps that reading the way it was written: the report matches on the
    // record and this consumes it.
    let recorder = match recorder {
        Some(recorder) => {
            recorder.push_audio(open.record_mut());
            Some(recorder)
        }
        None => None,
    };

    // **A correction worth saying out loud is one that is a decision rather
    // than a trim** — acquiring, re-acquiring, a tap, an octave — which is
    // `karakuri-cli`'s rule and is here for P-0094's reason: an operator who
    // cannot see the grid decide cannot tell a lock from a coincidence. A trim
    // happens on every frame once locked and says nothing.
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

/// What the tracker's three controls read this frame: the offset the open
/// session is holding, and which way the grid can still be moved an octave.
///
/// [`transport`]'s shape one group along the same row — a function of what this
/// program can see and of nothing the console could work out for itself.
///
/// The offset is `None` where nothing is open, and that is the state rather
/// than a default: an offset belongs to a session, and
/// `karakuri_environment::audio`'s `DEFAULT_LATENCY_OFFSET_MS` is where the
/// *next* session starts rather than a value anything is holding now. The
/// console draws no track at all for it, which is `View::audio`'s own rule one
/// control to the left.
///
/// The two octave halves are the range against the session tempo, which is
/// exactly what `Audio::octave` refuses on — `BeatLock::octave` is
/// `BPM_RANGE.contains(&(bpm * factor))` and nothing else — so the chip the
/// panel draws inert is the press the lock would turn down. The range is asked
/// for by name through `karakuri_environment::audio`, which is the door this
/// program takes its audio through; a `60.0..=200.0` written here would be a
/// second copy of the tracker's own bound.
///
/// The tempo is the session's oscillator, which is the number the transport row
/// draws and the number the lock multiplies: one reading, so the chip that is
/// drawn and the press that is refused cannot come apart.
///
/// Not `Audio::octave` asked twice, which is the obvious alternative and is
/// wrong twice over: it *performs* the move, and it needs a session, where both
/// halves are drawn on a console with no input open — the refusal that is
/// *drawn* is the range's, and the one for a room that is not being listened to
/// is said out loud by [`scaled`].
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
///
/// [`attached`]'s and [`nudged`]'s shape, and it is deliberately not beside
/// them in [`App::performed`]: those two are `Silent(NoRecord)`, so the line
/// `unwritten` prints after them is true and they fall through to it. These two
/// are `Owed(NotSettled)`, so the arm that calls this leaves as soon as it
/// answers — the reason is written at the call, and what the alternatives were
/// is
/// [ADR-0278](../../../docs/adr/0278-an-operation-no-record-can-be-written-for-leaves-the-window-before-it-is-written.md).
///
/// `Instant::now()` is read here rather than passed in, because the instant a
/// tap means is the instant it arrived and this is the last place that is still
/// true. `started` is the run's own origin and comes from the caller: it is
/// `App::started`, the one this program measures every tap against, and a
/// second origin taken here would put two taps on two clocks.
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

/// A tap on the beat, performed against the room this program is listening to,
/// and what to say about it.
///
/// # Why it does not go through `written`
///
/// Every other control on this panel emits an `Operation`, `written` turns it
/// into a `Record` and [`apply`] moves the deck with it — P-0090. A tap does
/// end in a record: `karakuri_environment::audio` writes a `Record::Tempo` for
/// it and applies it to the session's oscillator, which is the same record a
/// replay would hand the engine. What it cannot do is come out of `written`:
/// that function is a pure function of the operation and a reading, and a tap's
/// record is the *beat lock's* answer — the tapped tempo, the phase error
/// against the oscillator, the output lag — none of which a `Current` carries.
/// So `written(TapBeat)` answers `Owed(NotSettled)`, and routing this key
/// through [`App::performed`] would print *"nothing moved, and nothing here
/// decides it"* about a press that moved the grid.
///
/// That is a gap in `karakuri-operation-record` and it is named here rather
/// than papered over: the day a `Current` can carry a correction, this key
/// emits like every other control and this function goes. Until then it is
/// `karakuri-cli`'s own wiring, which is what the panel was asked to use.
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

/// The grid, an octave up or down, performed against the same session, and what
/// to say about it.
///
/// [`tapped`]'s note about `written` word for word: `ScaleGrid` is the other
/// half of that `Owed(NotSettled)` arm, and for the same reason.
///
/// Refused where the result would leave the trackable range, which is the
/// lock's call and not this file's — 60 to 200 BPM is under two octaves wide,
/// so at most one of the two directions is ever live and a control that undid
/// itself two seconds later would be worse than one that says no.
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
