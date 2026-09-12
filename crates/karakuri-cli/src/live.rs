use super::*;

/// How often the status line is printed. Not every frame: at 120 Hz that is a
/// line of stderr per 8 ms, which is unreadable and is I/O on the render thread
/// that nobody asked for.
pub(crate) const STATUS_INTERVAL: Duration = Duration::from_millis(500);

/// One press of a gain key. Linear and additive, because a fader is: the
/// operator wants the same physical move to mean the same amount everywhere,
/// not a proportion of wherever the slot happens to be.
pub(crate) const GAIN_STEP: f32 = 0.1;

/// Where a scheduled fade starts, and what to call it. A bar is four beats
/// here, which is an assumption rather than a measurement: nothing in the
/// signal bus knows a time signature, and four is what the `bar` signal already
/// means. A set in three would want this to be a dial, and would say so by
/// having a fade land in the wrong place — which is a better way to find out
/// than a setting nobody knew to change.
pub(crate) const QUANTA: [(f64, &str); 3] =
    [(4.0, "the next bar"), (1.0, "the next beat"), (0.0, "now")];

/// How long a scheduled fade lasts, in beats. A bar, half a bar, two bars, and
/// a cut — the four an operator reaches for, in the order they are reached for.
pub(crate) const FADE_BEATS: [f64; 4] = [4.0, 2.0, 8.0, 0.0];

/// The shapes a wipe can take, in cycle order, with the angle each runs at and
/// what to call it.
///
/// `None` first, so that a deck nobody has touched wipes with nothing and says
/// so rather than doing something. The rest are the four directions and the
/// iris — the ones a hand reaches for. An arbitrary angle is a dial, and a dial
/// with nowhere to show its value is a control an operator cannot read.
pub(crate) const MASK_SHAPES: [(MaskKind, f32, &str); 6] = [
    (MaskKind::None, 0.0, "off — `c` needs a shape"),
    (MaskKind::Linear, 0.0, "linear, left to right"),
    (
        MaskKind::Linear,
        std::f32::consts::FRAC_PI_2,
        "linear, bottom to top",
    ),
    (
        MaskKind::Linear,
        std::f32::consts::FRAC_PI_4,
        "linear, diagonal",
    ),
    (
        MaskKind::Linear,
        -std::f32::consts::FRAC_PI_4,
        "linear, the other diagonal",
    ),
    (MaskKind::Radial, 0.0, "an iris"),
];

/// One press of an opacity key. Additive for the same reason as [`GAIN_STEP`],
/// and clamped to `[0, 1]` where gain is not: opacity is a proportion of a
/// blend and there is no such thing as 1.4 of one, while gain is a level into
/// an HDR mix and values above 1.0 are ordinary.
pub(crate) const OPACITY_STEP: f32 = 0.1;

/// One press of an exposure key, as a factor. Multiplicative, because exposure
/// is: a stop is a ratio, and an additive step would be enormous at 0.1 and
/// invisible at 8.0. This is a quarter of a stop, near enough.
pub(crate) const EXPOSURE_STEP: f32 = 1.189_207;

/// The interactive bounds, and only the interactive ones. `-` and `=` nudge
/// inside them, because a key that steps has to stop somewhere and a stop it
/// cannot see is worse than one it can. `--exposure` is not held to them, which
/// is deliberate and is `exposure_positive_is_accepted_unclamped`'s own
/// sentence: a batch render asks for something extreme on purpose, and a flag
/// is read once by somebody who typed it rather than nudged into a corner. What
/// the flag refuses is what has no meaning at all — see [`clamp_exposure`] for
/// why zero and negative are neither clamped nor accepted anywhere.
///
/// This comment said the two shared a range and they never have. It was written
/// beside a constant pair pulled out so the bound would not drift, and the
/// drift was the sentence rather than the numbers.
pub(crate) const EXPOSURE_MIN: f32 = 1.0 / 64.0;
pub(crate) const EXPOSURE_MAX: f32 = 64.0;

/// Exposure is a multiplier before the tone map; zero is degenerate (always
/// black) and negative inverts an otherwise-positive HDR value into one no tone
/// mapper is specified for. Pulled out as a pure function so the bound is one
/// piece of logic instead of two copies that could drift, and so it is testable
/// without a `Live` or a GPU.
pub(crate) fn clamp_exposure(exposure: f32) -> f32 {
    exposure.clamp(EXPOSURE_MIN, EXPOSURE_MAX)
}

/// A demonstration that drives itself, as a list of `(seconds, key)`.
///
/// Every entry goes through [`Live::key`], the same function a keyboard
/// reaches, so what a watcher sees is what pressing those keys does and not a
/// second path that resembles it. Nothing here can do anything a person could
/// not.
///
/// It exists because a window is the only honest demonstration of a transport —
/// the scrub is a motion, and a still frame of it is a still frame — and
/// whoever is *describing* the feature is often not the one at the keyboard.
///
/// The order is the argument: engage first and let it sit, so it is clear that
/// engaging changes nothing; then scrub back a long way, hold, and let it run
/// back onto the grid.
pub(crate) const DEMO_SCRIPT: &[(f32, char)] = &[
    // Four seconds of the material as it is, for a before.
    (4.0, 'y'), // beat sync — the picture does not move, deliberately
    // Two bars back, a quarter beat at a time, over about a second and a half.
    (6.0, 'U'),
    (6.1, 'U'),
    (6.2, 'U'),
    (6.3, 'U'),
    (6.4, 'U'),
    (6.5, 'U'),
    (6.6, 'U'),
    (6.7, 'U'),
    (6.8, 'U'),
    (6.9, 'U'),
    (7.0, 'U'),
    (7.1, 'U'),
    (7.2, 'U'),
    (7.3, 'U'),
    (7.4, 'U'),
    (7.5, 'U'),
    // Held there for three seconds: the slot is two bars behind the room and
    // still locked to it, so it moves at the room's rate from where it is.
    // Then forward again, past where it was.
    (10.5, 'i'),
    (10.6, 'i'),
    (10.7, 'i'),
    (10.8, 'i'),
    (10.9, 'i'),
    (11.0, 'i'),
    (11.1, 'i'),
    (11.2, 'i'),
    (11.3, 'i'),
    (11.4, 'i'),
    (11.5, 'i'),
    (11.6, 'i'),
    (11.7, 'i'),
    (11.8, 'i'),
    (11.9, 'i'),
    (12.0, 'i'),
    (12.1, 'i'),
    (12.2, 'i'),
    (12.3, 'i'),
    (12.4, 'i'),
    // Back to free running, which prints the refusal `tempo` earns on material
    // that reads `beats` on its way past.
    (16.0, 'y'),
    // Then a fade out and back in, on the defaults — the next bar, four beats
    // — so what a watcher sees is the gesture as it ships rather than one
    // tuned to be visible. The gap between the press and the movement is the
    // quantum doing its job and is the thing worth watching for.
    (17.5, 'F'),
    (22.0, 'G'),
];

/// How long one pass through [`DEMO_SCRIPT`] lasts before it starts over. Past
/// the last entry, so the run ends free-running for a few seconds — the state
/// it began in, which is what makes the next pass legible as a repeat rather
/// than as something new.
pub(crate) const DEMO_LOOP_SECONDS: f32 = 27.0;

/// The two topologies, over geometry that does not change, as a list of
/// `(seconds, key)` on the same terms as [`DEMO_SCRIPT`].
///
/// The deck holds one L1 file paired with a sprite renderer in slot 0 and a
/// stroke renderer in slot 1, so showing each slot without the other is the
/// demonstration. What a watcher sees is the same cloud, in the same places, at
/// the same instant, drawn two ways.
///
/// It fades a slot out rather than auditioning the other one, and that is the
/// one thing here that changed. This script pressed `v` three times — the mix,
/// each slot alone, the mix again — until ADR-0240 retired *Choose what the
/// output shows*. The output is the mix now and always, so the way to see one
/// slot without the other is to take the other out of the mix: `F` on the
/// focused slot's fader, `G` to bring it back, with a digit before each to say
/// which slot. That is `Operation::FadeDeck` where it used to be `SetPreview`,
/// and it isolates a slot exactly as the audition did.
///
/// What it costs is that a fade is scheduled and an audition was not. A press
/// lands on the current grid rather than in the frame it arrives, so the
/// picture changes a beat or two after the entry that asked for it — which is
/// the same gap [`DEMO_SCRIPT`]'s `F` and `G` already have and say is worth
/// watching for. The seconds below leave room for it.
///
/// The mix comes first and last on purpose. Both slots composited is the state
/// that shows they are the same geometry — the strokes lie along the dots — and
/// each slot alone is what shows how different the two look when the other is
/// not there to anchor it.
///
/// Nothing here presses a key that only means something to someone who was told
/// what to expect. Each fade produces a visibly different frame on its own,
/// which is the property [`DEMO_SCRIPT`]'s first entry deliberately does not
/// have and has to say so.
pub(crate) const DEMO_LINES_SCRIPT: &[(f32, char)] = &[
    // Five seconds of the mix: strokes and sprites over each other. Then slot
    // 1's fader out, leaving slot 0 alone — sprites *and* strokes, from one
    // simulation.
    (5.0, '1'),
    (5.0, 'F'),
    // Slot 1 back, slot 0 out: strokes alone, the same elements.
    (10.0, 'G'),
    (10.0, '0'),
    (10.0, 'F'),
    // And back to the mix.
    (15.0, 'G'),
];

/// One pass through [`DEMO_LINES_SCRIPT`], with five seconds of the mix after
/// the last press before it starts over.
pub(crate) const DEMO_LINES_LOOP_SECONDS: f32 = 20.0;

/// Which demonstration `--demo` runs.
///
/// A named script rather than a flag, because there is now more than one thing
/// worth showing and a single `--demo` would have to pick. Each is a
/// demonstration harness and not a feature: both drive [`Live::key`], so
/// neither can do anything a person at the keyboard could not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Demo {
    /// The transport: beat sync engaged, scrubbed two bars back, held, and run
    /// forward past where it was.
    Transport,
    /// `topology lines`: one L1, drawn as sprites and as strokes.
    Lines,
}

impl Demo {
    pub(crate) fn from_name(name: &str) -> Option<Demo> {
        match name {
            "transport" => Some(Demo::Transport),
            "lines" => Some(Demo::Lines),
            _ => None,
        }
    }

    pub(crate) fn script(self) -> &'static [(f32, char)] {
        match self {
            Demo::Transport => DEMO_SCRIPT,
            Demo::Lines => DEMO_LINES_SCRIPT,
        }
    }

    pub(crate) fn loop_seconds(self) -> f32 {
        match self {
            Demo::Transport => DEMO_LOOP_SECONDS,
            Demo::Lines => DEMO_LINES_LOOP_SECONDS,
        }
    }

    /// The deck this demonstration needs, for a run that named no material.
    ///
    /// A demonstration that requires the operator to assemble the scene is not one.
    /// `--demo lines` is about two renderers over one geometry, and a watcher
    /// handed a one-slot deck sees the script fade the only thing in the mix out
    /// and back. Overridden the moment any `--set` is given, so this supplies a
    /// scene rather than imposing one.
    pub(crate) fn deck(self) -> Vec<(Named, Vec<Named>)> {
        match self {
            Demo::Transport => Vec::new(),
            // **Two slots, and it has to stay two.** A stack — one slot with
            // both renderers over one simulation — is what several renderers
            // over one geometry now costs, and it is the wrong shape *here*:
            // `DEMO_LINES_SCRIPT` fades one slot's fader out at a time so the
            // other is seen alone, and a fader is per **slot**. A one-slot
            // deck has nothing to take away, so the script would show the same
            // picture and then an empty frame, and the demonstration would
            // run, look like it worked, and demonstrate nothing — which is the
            // defect the doc above already names.
            //
            // Slot 0 carries the stack anyway, so what the demonstration shows
            // is the mix, then one simulation drawn both ways, then strokes
            // alone. `--set drift_shell.kir,soft_points.kir,drift_streaks.kir`
            // is the plain way to ask for a stack and needs no demonstration.
            Demo::Lines => vec![
                (
                    Named::bare("examples/drift_shell.kir"),
                    vec![
                        Named::bare("examples/soft_points.kir"),
                        Named::bare("examples/drift_streaks.kir"),
                    ],
                ),
                (
                    Named::bare("examples/drift_shell.kir"),
                    vec![Named::bare("examples/drift_streaks.kir")],
                ),
            ],
        }
    }
}

/// Every node of one slot's build: the layer it was sorted onto, its index
/// within that layer, and the hash of the source it was compiled from.
pub(crate) type Nodes = Vec<(&'static str, u32, karakuri_store::hash::Hash)>;

/// One press of the scrub keys, in beats. A quarter beat — a sixteenth of a bar
/// in four — which is small enough to place a hit by ear and large enough to
/// hear one press.
pub(crate) const SCRUB_BEATS: f64 = 0.25;

/// A level floor: a negative gain would subtract one slot's light from
/// another's, which is a blend mode rather than a level. Not ceilinged — the
/// pipeline is HDR and values above 1.0 are expected. Pure for the same reason
/// as [`clamp_exposure`].
///
/// `Deck::set_gain` floors too, and the two are not a duplicate. This one
/// decides what the record says, so a session replays the value that took
/// effect rather than one the engine quietly corrected; that one guards the
/// engine against every record it did not write, which is the whole of a
/// replay. Deleting either leaves a real hole.
pub(crate) fn clamp_gain(gain: f32) -> f32 {
    gain.max(0.0)
}

/// The tone map cycle `t` steps through. Pure so the cycle — and that it
/// returns to where it started — is checkable without a window.
pub(crate) fn next_tonemap(op: TonemapOp) -> TonemapOp {
    match op {
        TonemapOp::Clamp => TonemapOp::Reinhard,
        TonemapOp::Reinhard => TonemapOp::Aces,
        TonemapOp::Aces => TonemapOp::AgX,
        TonemapOp::AgX => TonemapOp::Clamp,
    }
}

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

    // A correction is worth saying out loud when it is a decision rather than a
    // trim: acquiring, re-acquiring, and a tap all move the grid at once, and an
    // operator who cannot see that happen cannot tell a lock from a coincidence.
    // Scalars only, so pushing it allocates nothing — which is why the tempo
    // half of the audio path can be recorded on a frame and the measurement half
    // cannot yet. See the note in `karakuri-environment`'s `session.rs`.
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

/// What a governor pass decided, said out loud.
///
/// One function because there is one thing to say. A replay governs too —
/// `Record::Residency` carries the request and never the effective level,
/// precisely so that the machine replaying re-derives it — and the first
/// version of that had its own smaller copy of these three loops, printing the
/// parked slots and not the summary. That is the shape this whole change exists
/// to remove: a second implementation that agrees until it does not.
pub(crate) fn report_governing(report: &karakuri_engine::governor::Report, why: &str) {
    eprintln!("{why}: {report}");
    for decision in report.parked() {
        eprintln!(
            "  slot {} parked, request held: {:?}",
            decision.slot, decision.reason
        );
    }
}

/// What this program does with an operation: the records it writes, or the
/// sentence saying it did nothing.
///
/// [`Live::performed`] takes the readings and this decides, so that the answer
/// a model is handed and the line a terminal is given are one string built
/// once. It is a free function rather than a method for the same reason
/// [`rewired`] is: it needs no `Live`, and a test can hold it against a real
/// call over the socket without a window or a GPU.
///
/// An operation that writes no record writes nothing here. This surface
/// performs an operation by converting it to records and reading them back —
/// there is no second arm — so `Written::Silent` and `Written::Owed` both mean
/// nothing on this run changed, and a caller that reported success for one
/// would be reporting a change it did not make
/// ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md):
/// a silently wrong answer loses to a loud failure). That is the whole of why
/// this is a `Result`: `Live::operate`'s printed line reaches an operator who
/// is at the terminal, and a model on `--mcp` is not.
///
/// The refusal names the operation and where it is answered
/// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)),
/// and it is the performer's rather than the gate's — which is
/// [ADR-0341](../../../docs/adr/0341-a-route-that-answers-is-built-and-a-send-that-ends-in-a-dialog-is-gap.md)'s
/// *a route that answers is a built route* read on the surface that has no
/// performer instead of the one that has one. `Silent::why` and `Owed::why` are
/// the reasons in the words the crate that decided them says them in, so
/// nothing is written down twice.
pub(crate) fn answered(operation: &Operation, current: &Current) -> Result<Vec<Record>, String> {
    match karakuri_operation_record::written(operation, current) {
        Written::Records(records) => Ok(records),
        Written::Silent(silent) => Err(format!(
            "`{}` was not performed and nothing on this run changed: {}, and this program \
             performs an operation by writing the records it converts to. `karakuri-cli` has \
             no control for this one — the instrument, `cargo run -p karakuri`, is the \
             surface that answers it.",
            operation.title(),
            silent.why()
        )),
        // **The reading, and not the control.** `Owed` says a record is owed
        // and could not be made here — a deck the run does not hold, or a
        // vocabulary question nobody has settled — so the sentence is that
        // reason and not *this program has no control for it*, which would be
        // false of an operation whose key is on this keyboard.
        Written::Owed(owed) => Err(format!(
            "`{}` was not performed and nothing on this run changed: {}",
            operation.title(),
            owed.why()
        )),
    }
}

/// The answer an operation that was performed goes back with.
///
/// One string, so that [`Live::run_operations`] and the test that drives it
/// over a socket say the same thing — and so that the sentence which says
/// *where a later answer lands* is beside the one that says nothing landed.
pub(crate) fn performed_at_the_frame(title: &str) -> String {
    format!(
        "`{title}` was performed on the frame it arrived on, where the same operation from a \
         key or a mapped control is performed. Anything it started rather than finished is \
         reported where it lands: ask `swap_outcome` for a rebuild, and a scheduled move \
         arrives on the grid."
    )
}

/// Whether `at` names a renderer of a slot that draws with `count` of them, and
/// the sentence if it does not. [`slot_in_range`]'s companion, returning the
/// refusal rather than a bool because both callers print it.
pub(crate) fn renderer_in_range(slot: usize, at: usize, count: usize) -> Result<(), String> {
    if at < count {
        Ok(())
    } else {
        Err(no_such_renderer(slot, at, count))
    }
}

pub(crate) struct Live {
    pub(crate) window: Arc<Window>,
    pub(crate) gpu: Gpu,
    /// Where a composed frame goes. The window is a sink rather than *the* output —
    /// see `docs/plugins.md`, where the others hang.
    pub(crate) sink: frame::WindowSink,
    pub(crate) present: Present,
    /// Every Set, whatever is being built to replace any of them, and the mix.
    /// Without `--watch` every slot is a `HotSwap::fixed` and there is no worker at
    /// all, so the frame loop below is the same code either way.
    pub(crate) deck: Deck,
    pub(crate) look: Look,
    /// The slot the gain keys act on. There is no on-screen UI, so this is printed
    /// on every change and marked in the status line.
    pub(crate) focus: usize,
    pub(crate) clock: Clock,
    /// The audio input, the beat lock, and the operator's latency offset. `None`
    /// without `--audio-in`, and then nothing in the frame path below changes at
    /// all — which is the property the whole slice is about.
    pub(crate) audio: Option<audio::Audio>,
    /// The control surface, when `--midi-in` asked for one. Every operation it
    /// produces ends in the same record a key press ends in — see [`crate::midi`] —
    /// so nothing in the frame path below changes at all when this is `None`.
    pub(crate) midi: Option<midi::Surface>,
    /// The tempo source, when `--tempo-source` asked for one. `None` and nothing in
    /// the frame path below changes at all — the grid comes from `--bpm`, the
    /// tracker and the tap keys, exactly as it did before this existed. That is the
    /// same property `audio` and `midi` have and it is the one worth keeping.
    pub(crate) tempo_source: Option<tempo_source::Source>,
    /// What each slot's watcher built, by build id, until the swap that build
    /// produced lands. Not a log: an entry is taken when its build lands or dropped
    /// when a newer one supersedes it.
    pub(crate) rebuilds: Option<std::sync::mpsc::Receiver<watch::Built>>,
    pub(crate) pending_builds: std::collections::HashMap<u64, watch::Built>,
    /// What each slot is running. Seeded before the first frame from the text the
    /// compile read, so there is exactly one way to answer the question a live save
    /// asks — see [`Running`].
    pub(crate) running: Running,
    /// Where each slot's files were at launch, one entry per node, in the order
    /// they were spelled.
    ///
    /// The names, and the bytes each node was compiled from. A name belongs to the
    /// use rather than to the procedure, so `--set veil=shell.kir` is a fact about
    /// the command line that no rebuild restates and no hash carries, and
    /// `Live::save_set` zips these onto the hashes by position.
    ///
    /// Not the paths, which is the distinction that matters. This used to answer
    /// "what is this slot running" by re-reading `named.path`, and that was a
    /// second answer to a question [`Running`] already held. What is here now is
    /// [`Placed::source`] — the text the compile read — and it is not a second
    /// answer but the *first*: every hash in `Running` was derived from it, and a
    /// save hands the same buffer to the store so the file it writes resolves.
    /// There is one derivation, and this is where its input lives.
    ///
    /// Empty for a slot filled straight from a Set file without `--watch` or
    /// `--mcp`, which has no files behind it at all — see where `placed` is built.
    pub(crate) startup: Vec<Vec<Placed>>,
    /// The Set file this run was loaded from, for the one refusal that has to name
    /// it: a slot filled from a file with nothing watching it has no sources to
    /// save, and an operator asking why is owed the id and the flag that would
    /// change the answer.
    pub(crate) loaded_set: Option<String>,
    /// Which node fills each declared input slot, for the whole run.
    ///
    /// Read from the arguments rather than from the live Set, which is the one
    /// place `Live::save_set` does that and needs its reason. An edge is consumed
    /// where a Set is *built* and is not kept on it, so there is nothing to read
    /// back. So this is a copy of a value, not a second copy of a rule, which is
    /// the distinction `saving_capacities` was fixed over.
    ///
    /// It moves during a run, and this is where it moves. That sentence used to
    /// read *no key and no MCP tool rewires a `uses` slot*; `wire_input` does, and
    /// [`rewired`] is what it reaches. There is still no second answer to what the
    /// run is wired with — this list is the one, a rewiring replaces one entry of
    /// it and hands the whole of it to the slot's watcher through [`Aiming`], so
    /// the list a rebuild restates and the list a save records stay one list.
    ///
    /// One list for the run and not one per slot, which is what makes the key
    /// `(node, slot)` and not `(deck, node, slot)` — see [`rewired`], and
    /// `Wiring::edges` for why an edge naming a node a Set has not got is simply
    /// passed over.
    ///
    /// A limit worth naming: a rewiring re-aims only the slot the request named, so
    /// any *other* watcher goes on restating the list it was last aimed with until
    /// it is itself re-aimed. That is invisible unless two slots hold nodes of the
    /// same name, which is also the only case where one entry in this list was ever
    /// about two Sets. Re-aiming every watcher instead would recompile the whole
    /// deck for one edge, which is a far louder wrong answer.
    pub(crate) edges: Vec<karakuri_engine::set::Edge>,
    /// Where each slot's watcher can be re-pointed, one entry per slot and `None`
    /// for a slot with no watcher — a run without `--watch`, and every
    /// `HotSwap::fixed`. See [`Aiming`]: this is how an edge written over MCP
    /// reaches the thing that rebuilds with it.
    pub(crate) aims: Vec<Option<Aiming>>,
    /// The store root a live save writes into. The *root* and not an open store:
    /// every part of a save that touches a disk happens on the thread that does it
    /// — see [`Save::run`].
    pub(crate) store_root: PathBuf,
    /// Where a save reports back. One thread per save writes into the sender's
    /// clone; the frame loop drains the receiver, which is the shape `rebuilds`
    /// already has and for the same reason: an outcome arrives when it arrives, and
    /// a frame must not wait for it.
    pub(crate) save_tx: std::sync::mpsc::Sender<Saved>,
    pub(crate) saves: std::sync::mpsc::Receiver<Saved>,
    /// How many saves have been started and not yet reported back. The threads are
    /// detached, so this is the only thing that knows a file is still being written
    /// — see [`Live::awaited_saves`], which is why anything counts them at all.
    pub(crate) saves_in_flight: usize,
    /// The MCP server's half of the channel, when `--mcp` asked for one. Told what
    /// the swap machinery said, and nothing else — see [`crate::mcp`].
    pub(crate) mcp: Option<mcp::Reporter>,
    /// Scratch for [`midi::Surface::take`], owned so the frame path allocates
    /// nothing. Empty on every frame nothing was touched.
    ///
    /// Nothing a map line can name carries a heap payload — every one of them is
    /// scalars — so a fader sweep reuses this buffer and touches no allocator,
    /// which is what the render-thread rule asks of it.
    pub(crate) operations: Vec<Operation>,
    /// The musical grid a scheduled fade starts on — see [`QUANTA`]. State on the
    /// operator rather than in the record: what reaches the stream is the resolved
    /// beat count, so this is a setting for the hand and not for the timeline.
    pub(crate) quantum: f64,
    /// How long a scheduled fade lasts, in beats. Same reasoning.
    pub(crate) fade_beats: f64,
    /// The shape the next wipe uses, and which way it runs. Not a slot's mask: this
    /// is what `c` will *give* a slot, where the slot's own is deck state and
    /// travels in the record stream.
    pub(crate) mask_kind: MaskKind,
    pub(crate) mask_angle: f32,
    /// When the session started, so a tap has an origin to be measured from.
    pub(crate) started: Instant,
    pub(crate) status_at: Instant,
    pub(crate) frames_since_status: u32,
    /// Writes the timeline, when `--record-session` asked for one. The frame path
    /// pushes into it and never blocks or allocates — see [`crate::session`].
    pub(crate) recorder: Option<session::Recorder>,
    /// Which demonstration is running and how far into its script, or `None` when
    /// `--demo` was not given and nothing drives itself.
    pub(crate) demo: Option<(Demo, usize)>,
    /// When the current pass through [`DEMO_SCRIPT`] started. The script loops, so
    /// this is not [`Live::started`]: that one is the session's origin and a tap is
    /// measured from it.
    pub(crate) demo_started: Instant,
    /// Reused by the status line. Printing at all on this thread means locking
    /// stderr, but there is no reason for it to mean a fresh allocation twice a
    /// second as well.
    pub(crate) status: String,
}

impl Live {
    /// The window changed size. Nothing that is rendered changes.
    ///
    /// This used to resize the HDR target and every deck slot as well, because the
    /// window's size *was* the canvas. Two things came of that, and both are gone
    /// with it: dragging a window reallocated every slot's target once per frame of
    /// the drag — a GPU allocation on the render thread, which is the one thing
    /// this engine's frame path forbids — and what a run rendered depended on how
    /// big its window happened to be, so the same session replayed at a different
    /// size with nothing saying which was the performance. All that is left here is
    /// the swapchain, which has to follow the window because it *is* the window.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.sink.resize(&self.gpu.device, width, height);
    }

    /// Resize the window so the canvas lands in it one texel to one texel.
    ///
    /// The preview is fitted, so an OBS window capture of it would otherwise pick
    /// up the bars and a scale — and a capture that is neither the canvas nor a
    /// clean crop of it is worse than useless downstream. After this the window
    /// contains the canvas exactly, and `letterbox` becomes the identity.
    ///
    /// A request, not a guarantee, and the difference is printed. A 1080-tall
    /// canvas cannot get a 1080-tall content window on a 1080-tall display — there
    /// is a menu bar or a taskbar in the way — so the manager clamps it, and an
    /// operator setting up a capture has to be told that rather than told "1:1".
    ///
    /// `request_inner_size` returns the granted size immediately on the platforms
    /// where the manager decides and `None` where a `Resized` event will follow.
    /// Taking the returned value matters on the first kind: no event arrives, so
    /// nothing else would ever reconfigure the swapchain, and a swapchain that
    /// disagrees with its window does not fail — `set_viewport` is not validated
    /// against the attachment — it just draws the wrong picture, silently.
    fn snap_to_canvas(&mut self) {
        let (w, h) = self.present.size();
        match self
            .window
            .request_inner_size(winit::dpi::PhysicalSize::new(w, h))
        {
            Some(granted) => {
                self.resize(granted.width, granted.height);
                if (granted.width, granted.height) == (w, h) {
                    eprintln!("window: {w}x{h}, 1:1 with the canvas");
                } else {
                    eprintln!(
                        "window: asked for {w}x{h} and got {}x{} — a capture of this is \
                         not the canvas",
                        granted.width, granted.height
                    );
                }
            }
            // A `Resized` is coming, and it reports what was actually granted.
            None => eprintln!("window: asked for {w}x{h}, 1:1 with the canvas"),
        }
    }

    /// Press whatever [`DEMO_SCRIPT`] is due, if this run is driving itself.
    ///
    /// Wall clock rather than frame count, because what is being demonstrated is a
    /// performance and a performance happens in seconds. That makes the demo *not*
    /// reproducible frame for frame, which is fine and is worth saying: it is a
    /// thing to look at, not a thing to diff. Everything it presses goes through
    /// [`Live::key`], so it can do nothing a person at the keyboard could not.
    fn run_demo(&mut self) {
        let Some((demo, next)) = self.demo else {
            return;
        };
        let script = demo.script();
        let elapsed = self.demo_started.elapsed().as_secs_f32();
        let mut at = next;
        while let Some((due, key)) = script.get(at) {
            if elapsed < *due {
                break;
            }
            let key = Key::Character(key.to_string().into());
            self.key(&key);
            at += 1;
        }
        // **It loops**, because a demonstration nobody happened to be looking
        // at is a demonstration that did not happen. Every script is under half
        // a minute and starts over, so glancing at the window at any moment
        // eventually shows the thing.
        if at >= script.len() && elapsed >= demo.loop_seconds() {
            self.demo_started = Instant::now();
            at = 0;
        }
        self.demo = Some((demo, at));
    }

    /// Whatever the control surface did since the last frame, as the same
    /// operations a key press and a console fader name.
    ///
    /// The whole of the MIDI connection, and there is almost nothing in it: a
    /// mapped message *is* an [`Operation`], so this hands each one to
    /// [`Live::operate`] and that is the connection. A surface can do nothing a key
    /// cannot because both end in the same record, and a session recorded from one
    /// replays with neither attached.
    ///
    /// This used to be a match over eight `Action`s claiming that a control added
    /// to one and not the other does not compile. Against a fifty-variant
    /// vocabulary that claim would be false — a router arm nobody wrote is a
    /// wildcard nobody notices. The guarantee is now where it is true:
    /// `karakuri_operation_record::written` is one exhaustive match over all fifty,
    /// so an operation nobody has said what to do with stops the build there.
    ///
    /// [`Operation::TapBeat`] is handled here and it is the only one, for a reason
    /// that is visible rather than incidental: a tap moves the beat tracker rather
    /// than writing a value, `written` answers `Owed::NotSettled` for it, and
    /// `Live::operate` would print that gap instead of tapping. `Live::tap` is what
    /// owns the tracker and what the `b` key reaches, so `note -> tap` goes on
    /// doing exactly what it did. The day the record a tap owes is settled, this
    /// arm is what goes.
    ///
    /// Nothing here prints. The old arms ended in `set_gain` and its neighbours,
    /// each of which reports what it did — which on a fader sweep is an `eprintln!`
    /// per MIDI message inside a frame, several hundred a second, and is the
    /// blocking write per message `crate::midi`'s own "once per control" rule
    /// exists to prevent. What a surface moved is read back from the deck (`s`),
    /// not narrated per message.
    ///
    /// Before the tick, so a fader move lands on the frame it arrived for rather
    /// than the one after — the same placement `run_demo` has, and for the same
    /// reason.
    fn run_surface(&mut self) {
        let Some(surface) = &mut self.midi else {
            return;
        };
        // Into the owned scratch, then out of `self`'s borrow, so the calls
        // below can take `&mut self`. Nothing allocates: both vectors are
        // reused and `take` clears rather than replaces.
        let mut operations = std::mem::take(&mut self.operations);
        // The slot check is the router's — it is where the "say it once"
        // machinery already is, and once per *message* would be a blocking
        // write per message on this thread. See `crate::midi`.
        //
        // **And the deck, for the one target the map cannot finish on its
        // own**: `cc -> param N M` names a *position* in a deck's published
        // interface, which becomes a key only against the Set that is in the
        // deck (ADR-0268, ADR-0336). `midi::Decks` is that reading, and it is
        // the same one the panel makes — a map file means one thing in both
        // programs or it means nothing.
        surface.take(
            self.deck.slot_count(),
            &karakuri_environment::midi::Decks(&self.deck),
            &mut operations,
        );
        for operation in &operations {
            match operation {
                Operation::TapBeat => self.tap(),
                other => self.operate(other),
            }
        }
        self.operations = operations;
        // **And the surface is shown where the deck ended up** — MIDI out, on
        // the frame the change lands and after the frame's operations have
        // been applied, so a motorised fader follows the value the deck holds
        // rather than the one it was asked for.
        //
        // **It does not wait**: `Surface::show` queues into a bounded channel
        // and drops when it is full rather than blocking this thread, which is
        // the same rule every other thing this frame does (P-0094,
        // `crate::midi`). A run with no output port costs one branch.
        //
        // **Every source is shown, not just the surface's own.** A key press,
        // a model over `--mcp` and a transition move the deck too, and the
        // whole point of MIDI out is that two things can move a fader.
        if let Some(surface) = &mut self.midi {
            surface.show(&karakuri_environment::midi::Lit {
                deck: &self.deck,
                exposure: self.look.exposure,
            });
        }
    }

    /// What a model has asked for since the last frame.
    ///
    /// Beside [`Live::run_surface`] and on the same terms: a surface is polled at
    /// the top of a frame and every request it produces ends in the method a key
    /// press ends in. That is what makes `--mcp` a third pair of hands rather than
    /// a second way to do anything.
    ///
    /// Collected out of the borrow before any of it is acted on, exactly as the
    /// MIDI operations are, because every arm below takes `&mut self`. Nothing is
    /// allocated on a frame that was asked for nothing: collecting an empty
    /// iterator makes no allocation.
    ///
    /// Here rather than beside the swap drain below `frame::compose`, which was the
    /// other candidate: a client asking to keep what is playing should not be
    /// waiting on a swapchain, and nothing a request reaches needs the GPU. When
    /// that was written a frame with no surface returned before the drain, so it
    /// *was* waiting on one; a frame no longer returns early at all, and being
    /// above `frame::compose` is what the argument was always about.
    ///
    /// [`Live::finished_saves`] is here for the same reason and used to be down
    /// there, which meant this argument was made and then half applied: the request
    /// was taken above the early returns and its *answer* was withheld below them.
    /// See the comment at the head of [`Live::frame`].
    fn run_requests(&mut self) {
        let Some(mcp) = &self.mcp else {
            return;
        };
        let asked: Vec<mcp::SaveRequest> = mcp.saves().collect();
        // **Both channels drained before either is acted on**, for the borrow
        // reason above and for a second one: they are two queues by design —
        // see `mcp::Reporter::wires` — so a deck being saved to a slow disk
        // cannot delay a rewiring, and taking them in one pass is what keeps
        // that true on this side too.
        let wires: Vec<mcp::WireRequest> = mcp.wires().collect();
        // **And the operations, on the third channel and for the same two
        // reasons.** This run serves the same `mcp::serve` the panel does, so
        // its `operate` tool reaches this loop and not another one — a channel
        // this program did not drain would answer every call *the render loop
        // had not taken this operation*, which is true and is not what this
        // program is.
        let operations: Vec<mcp::OperateRequest> = mcp.operations().collect();
        for request in asked {
            self.save_set(Asked::Model, request.slot, request.id, Some(request.reply));
        }
        self.rewire(wires);
        self.run_operations(operations);
    }

    /// Every operation a model named since the last frame, performed where a mapped
    /// control's operation is performed.
    ///
    /// [`Live::run_surface`]'s own two lines, and they are two lines rather than a
    /// call into it because a map has a surface to poll and this has a channel to
    /// drain. What is shared is what matters: [`Live::operate`] is where a key
    /// press and a MIDI message end, and it is where this ends, so a model's
    /// `SetGain` on this program is the same write as a knob's
    /// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// Already audited. `karakuri_operation::gate` ran on the server's own thread —
    /// the one call ADR-0235 puts the mechanism on — so nothing is judged again
    /// here.
    ///
    /// Answered once, at the frame it was performed on, which is
    /// [`mcp::WireRequest`]'s third point one route along: what a rebuild or a
    /// scheduled move started here comes to is reported where it lands, and a tool
    /// that waited for it would hold a connection open across a transition.
    ///
    /// An operation this program cannot perform is refused rather than answered
    /// `ok`. `operate` is the panel's tool as much as this one's, and the two
    /// surfaces do not perform the same set: `crates/karakuri`'s `App::operated`
    /// calls the window's own press arms for a star, a projector, a recording and a
    /// kept procedure
    /// ([ADR-0341](../../../docs/adr/0341-a-route-that-answers-is-built-and-a-send-that-ends-in-a-dialog-is-gap.md)),
    /// and this program has none of them — it has keys, a MIDI map and the records
    /// they write. So every operation whose conversion writes no record does
    /// nothing here, and [`answered`] hands back the sentence that says so, which
    /// goes to the client as the call's error and to the terminal through
    /// [`refused`]. Reporting *performed* for it is the plausible wrong answer
    /// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
    /// is written against, and ADR-0334 refused it in as many words for the panel —
    /// *"a call answered `ok` for work that did not happen"* — one surface before
    /// it was this one's turn.
    ///
    /// [`Operation::TapBeat`] is the one arm that is not a conversion, and it is
    /// here for [`Live::run_surface`]'s reason: a tap moves the beat tracker,
    /// `written` answers `Owed::NotSettled` for it, and [`Live::tap`] is the
    /// performer this surface does have.
    fn run_operations(&mut self, asked: Vec<mcp::OperateRequest>) {
        for mcp::OperateRequest { operation, reply } in asked {
            let title = operation.title();
            let done = match &operation {
                Operation::TapBeat => {
                    self.tap();
                    Ok(())
                }
                other => self.performed(other),
            };
            match done {
                Ok(()) => reply.settled(Ok(performed_at_the_frame(title))),
                Err(said) => refused(Some(reply), said),
            }
        }
    }

    /// Every edge asked for since the last frame, written and answered here, on
    /// this frame.
    ///
    /// The decisions are [`rewired`]'s and are written there, because none of them
    /// needs a `Live`. What is here is the two things that do: the deck's own slot
    /// count, which is the only thing that knows how many slots there are, and the
    /// answer going back to whoever asked.
    ///
    /// Answered once, at the frame it was applied on, which is
    /// [`mcp::WireRequest`]'s third point. Not at the swap: what the *build* made
    /// of the edge is `swap_outcome`'s answer, as it is for every other rebuild,
    /// and a tool that waited for thirty judged frames would hold a connection open
    /// across a transition.
    ///
    /// One sentence for both audiences, which is [`refused`]'s rule: what the
    /// terminal is told and what the client is handed are the same words, so the
    /// second cannot be right on the day it is written and wrong at the next
    /// correction.
    fn rewire(&mut self, asked: Vec<mcp::WireRequest>) {
        if asked.is_empty() {
            return;
        }
        let mut wires = Vec::with_capacity(asked.len());
        let mut replies = Vec::with_capacity(asked.len());
        for mcp::WireRequest { slot, edge, reply } in asked {
            wires.push((slot, edge));
            replies.push(reply);
        }
        let said = rewired(
            &wires,
            &mut self.edges,
            &mut self.aims,
            self.deck.slot_count(),
        );
        for (reply, said) in replies.into_iter().zip(said) {
            match &said {
                Ok(line) | Err(line) => eprintln!("{line}"),
            }
            reply.settled(said);
        }
    }

    /// A key press. Returns true if it was a request to quit.
    ///
    /// Every branch prints what it did. There is no on-screen UI, and a control
    /// that changes something invisible — a gain on an off-air slot, an
    /// operator on a dark frame — is indistinguishable from a control that is
    /// broken.
    ///
    /// # Every key names an operation, and they fall in four groups
    ///
    /// [`Live::run_surface`] takes a mapped message straight to
    /// [`Live::operate`], because every operation a map line can produce is one
    /// `karakuri_operation_record::written` converts. A keyboard is not that
    /// shape, and this is the survey rather than an intention: of the
    /// thirty-nine keys below, twenty-one name an operation whose record
    /// converts, three name one whose record is owed, twelve name one that
    /// writes no record at all, and three name nothing in the vocabulary.
    ///
    /// - Its record converts, so it goes through [`Live::operate`].
    ///   `space` and `w` (`SetResidency`), `[`, `]` and `\` (`SetGain`),
    ///   `;` and `'` (`SetOpacity`), `m` (`SetBlendMode`),
    ///   `t` (`SetTonemap`), `-`, `=` and the backquote (`SetExposure`), `u`
    ///   and `i` (`ScrubDeck`), `y` (`SetSync`), `f` and `g` (`FadeDeck`),
    ///   `x` (`Crossfade`), `r` (`SelectRenderer`), `c` (`Wipe`). A key and a
    ///   mapped pad reach the deck by one
    ///   derivation, which is the whole of why the two surfaces cannot drift.
    ///   Which way to step is still the keyboard's — a cycle and a nudge are
    ///   translations a surface makes, never operations
    ///   (`docs/principles/0090-a-surface-offers-it-never-decides.md`).
    /// - Its record is owed, so it keeps its present path — each with the
    ///   reason written at the function it ends in: `b` ([`Live::tap`]), `,`
    ///   and `.` ([`Live::shift_octave`]). All three
    ///   operations answer `Owed::NotSettled`, and the day one is settled its
    ///   function is what goes — the promise `run_surface`'s `TapBeat` arm
    ///   already carries. Routing one through `operate` today would print the
    ///   gap where the gesture used to happen, which is the one thing a change
    ///   of route may not do.
    ///
    ///   It was eight, and the five that left are what the promise looks
    ///   like kept three times. `Operation::SetSync` was owed for the
    ///   engine's anchor clamp; the clamp turned out to be the identity on
    ///   every tempo an oscillator can report, the conversion took a session
    ///   tempo as a reading, and [`Live::cycle_sync`] moved to `operate` with
    ///   its record-building deleted rather than kept in step. `FadeDeck`,
    ///   `Crossfade` and `SelectRenderer` went the same way and took a whole
    ///   function with them: the quantum and the length `n` and `j` cycle are
    ///   this surface's, `Current::transition` is where they are handed
    ///   over, and `Live::fade_slot` is gone rather than kept beside the
    ///   conversion. `c` was the one that stayed and is the fifth to go,
    ///   because what a wipe carried beyond a fade turned out to be unassigned
    ///   rather than missing too: the shape its front takes is the third of
    ///   the settings `z`, `n` and `j` write and travels with the other two,
    ///   and the soft edge is read off the mask on the deck being wiped in.
    ///   [`Live::wipe`] keeps only the two refusals and the line it prints.
    ///
    ///   What is left is the beat tracker, and it is a different shape. A
    ///   tap and an octave shift do not want a reading nobody hands over; they
    ///   want the beat lock's answer, which no value a surface holds
    ///   determines — see [`Live::tap`].
    /// - It writes no record, and this surface is what has to perform it.
    ///   `0`–`3` (`SelectDeck`), `z`, `n` and `j` (`SetTransition`), `a`
    ///   (`SizeWindow`), `k` (`SaveSet`), `o` and `p` (`SetLatencyOffset`),
    ///   `esc` (`Quit`). These cannot route through `operate` either, and
    ///   that is a fact about `Silent` rather than an omission: `operate`
    ///   turns an operation into the records it writes and applies those, so an
    ///   operation that writes none would print [`answered`]'s refusal and the
    ///   key would do nothing. What most of them change is a surface's own
    ///   state, and this surface is the only thing holding it.
    ///
    ///   Two of them are reachable over `--mcp` and are refused there, in
    ///   that same sentence: `Operation::SetLatencyOffset` and
    ///   `Operation::Quit` are `Sayable::Operable`, and a key that nudges is
    ///   not a performer for an operation that names a value. See
    ///   [`Live::run_operations`].
    /// - The vocabulary does not name it at all. `S` prints the status line
    ///   and `h`/`?` print [`BINDINGS`]. Neither has a row on
    ///   `docs/manual/operations.html`, which is the specification for which
    ///   operations exist — so neither is an operation anybody has
    ///   specified. Left as found and reported, because a row invented here
    ///   would be a specification written from the implementation.
    ///
    ///   *(Under ADR-0346, superseding ADR-0220, the CLI and GUI keyboard
    ///   mappings converge onto the unified specification in
    ///   `docs/manual/operations.html`. The interim scaffolding divergence
    ///   on single-letter keys is resolved by migrating the colliding CLI
    ///   keys to uppercase keys `F`, `G`, `Z`, `R`, `N`, `U`, `S`, while `o`
    ///   and `p` agree on both keyboards for audio latency offset. See
    ///   `docs/adr/0346-the-gui-and-cli-keymaps-diverged-in-scaffolding-and-converge-on-the-operations-page.md`.)*
    pub(crate) fn key(&mut self, key: &Key) -> bool {
        match key.as_ref() {
            Key::Named(NamedKey::Escape) => return true,
            Key::Named(NamedKey::Space) => self.toggle_focused(),
            Key::Character(s) => match s.chars().next().unwrap_or('\0') {
                c @ '0'..='3' => self.focus_slot(c as usize - '0' as usize),
                '[' => self.nudge_gain(-GAIN_STEP),
                ']' => self.nudge_gain(GAIN_STEP),
                '\\' => self.set_gain(self.focus, 1.0),
                ';' => self.nudge_opacity(-OPACITY_STEP),
                '\'' => self.nudge_opacity(OPACITY_STEP),
                'm' => self.cycle_blend(self.focus),
                'F' => self.fade(0.0),
                'G' => self.fade(1.0),
                'x' => self.crossfade(),
                'c' => self.wipe(),
                'Z' => self.cycle_mask(),
                'R' => self.cycle_renderer(),
                'N' => self.cycle_quantum(),
                'j' => self.cycle_fade_beats(),
                't' => self.cycle_tonemap(),
                '-' => self.set_exposure(self.look.exposure / EXPOSURE_STEP),
                '=' => self.set_exposure(self.look.exposure * EXPOSURE_STEP),
                '`' => self.set_exposure(1.0),
                'w' => self.toggle_priming(self.focus),
                'y' => self.cycle_sync(),
                'U' => self.scrub(-SCRUB_BEATS),
                'i' => self.scrub(SCRUB_BEATS),
                'b' => self.tap(),
                ',' => self.shift_octave(0.5),
                '.' => self.shift_octave(2.0),
                'o' => self.nudge_latency_offset(-audio::LATENCY_OFFSET_STEP_MS),
                'p' => self.nudge_latency_offset(audio::LATENCY_OFFSET_STEP_MS),
                'a' => self.snap_to_canvas(),
                // The focused slot, a stamped name, and nobody waiting: a hand
                // has one slot in front of it, cannot type a name, and is
                // reading the terminal.
                'k' => self.save_set(Asked::Operator, self.focus, None, None),
                'S' => self.print_status(),
                'h' | '?' => eprint!("{BINDINGS}"),
                _ => {}
            },
            _ => {}
        }
        false
    }

    fn focus_slot(&mut self, slot: usize) {
        if !slot_in_range(slot, self.deck.slot_count()) {
            eprintln!("{}", no_such_slot(slot, self.deck.slot_count()));
            return;
        }
        self.focus = slot;
        let addr = EngineSlot(slot as u8);
        eprintln!(
            "focus slot {slot} — {}, gain {:.2}, t {:.2}s",
            residency_name(self.deck.residency(addr), self.deck.is_parked(addr)),
            self.deck.gain(addr),
            self.deck.slot(addr).set().time()
        );
    }

    /// On air and off again. `Allocated` frees nothing and resets nothing, so the
    /// `t` printed on the way out is the `t` printed on the way back in — which is
    /// the whole property, and printing both ends is the only way to see it without
    /// a debugger.
    fn toggle_focused(&mut self) {
        self.toggle_on_air(self.focus);
    }

    /// On air and off again, for a named slot.
    ///
    /// The toggle is the key's affordance and not an operation, which is why the
    /// slot is a parameter and the focus is filled in by the caller: what reaches
    /// the deck is `SetResidency` naming one of three
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`). A control
    /// surface says which state it wants on the line and does not come through here
    /// at all — it used to, and the state it landed in was this function's to
    /// decide.
    fn toggle_on_air(&mut self, slot: usize) {
        let addr = EngineSlot(slot as u8);
        let t = self.deck.slot(addr).set().time();
        match self.deck.residency(addr) {
            Residency::Live => {
                self.operate(&Operation::SetResidency {
                    deck: slot as u8,
                    residency: mix::residency(Residency::Allocated),
                });
                eprintln!("slot {slot} off air — allocated, holding t {t:.2}s");
            }
            // Priming is warming out of sight and is still off air, so space
            // does the same thing to it: puts it on, at whatever `t` it has
            // warmed to.
            Residency::Allocated | Residency::Priming => {
                self.operate(&Operation::SetResidency {
                    deck: slot as u8,
                    residency: mix::residency(Residency::Live),
                });
                eprintln!("slot {slot} on air — resuming at t {t:.2}s");
            }
        }
    }

    /// Ask the focused slot to warm out of sight, or withdraw the request.
    ///
    /// A request, not a command — the governor decides whether it is granted, and
    /// the report printed here says which. Whether it takes effect is exactly the
    /// thing the operator cannot otherwise see: a refused request leaves the slot
    /// at Allocated, which is what an untouched slot looks like too.
    ///
    /// Live slots are left alone. Priming is off-air warming, so asking a slot on
    /// air to prime could only mean taking it off air, and that is what space is
    /// for.
    fn toggle_priming(&mut self, slot: usize) {
        let addr = EngineSlot(slot as u8);
        if self.deck.residency(addr) == Residency::Live {
            eprintln!("slot {slot} is on air — priming is off-air warming; take it off with space");
            return;
        }
        let requested = self.deck.requested_residency(addr);
        let want = if requested == Residency::Priming {
            Residency::Allocated
        } else {
            Residency::Priming
        };
        // Said before the record is applied, because applying it runs a
        // governor pass that prints what it decided — and the decision reads as
        // an answer to a question the operator has not seen asked otherwise.
        eprintln!(
            "slot {slot} {}",
            if want == Residency::Priming {
                "asked to warm off air"
            } else {
                "prime request withdrawn"
            }
        );
        self.operate(&Operation::SetResidency {
            deck: slot as u8,
            residency: mix::residency(want),
        });
    }

    /// Take up what a slot is now playing, and say so in the stream if a session is
    /// being recorded.
    ///
    /// `landed` is the id of the build that went in, and there is no other case: a
    /// swap is the only event that changes what a slot holds (ADR-0316). It used to
    /// take an `Option`, whose `None` was a rollback, and the version that came
    /// back had to have been remembered because the stream would never name it
    /// again.
    ///
    /// The bookkeeping is unconditional and the record is not, and the two used to
    /// be one function that began by returning when there was no recorder. What a
    /// slot is playing was therefore a fact only a recorded run had — and
    /// `Live::save_set` needs exactly that fact in the ordinary `--watch` case,
    /// where nothing is being recorded. Splitting it is the whole of what widening
    /// `watch::Watch::stored` is for on this side of the channel.
    fn took_up(&mut self, slot: usize, landed: u64) {
        // Drained here rather than per frame: the channel only has anything in
        // it when a build has just been requested, and this runs when one has
        // just landed.
        if let Some(rx) = &self.rebuilds {
            while let Ok(built) = rx.try_recv() {
                self.pending_builds.insert(built.id, built);
            }
        }

        // **A build with nothing in `pending_builds` is one whose sources the
        // watcher could not store**, which it said at the time. It is handed to
        // `landed` as `None` rather than returned on, because the swap happened
        // either way: a slot that took a version nobody can name is a slot with
        // no address, not a slot still on its old one. See [`Running::landed`],
        // which is where that used to go wrong.
        let built = self.pending_builds.remove(&landed);
        let Some(nodes) = self.running.landed(slot, built.map(|built| built.nodes)) else {
            // A slot with nothing in the store behind it — one filled from a Set
            // file with nothing watching it, or one that has just taken a build
            // whose sources could not be stored, which was said at the time.
            // There is no hash to name, so there is nothing this could record.
            return;
        };
        self.record_procedure(slot, &nodes);
    }

    /// Say what a slot is playing, now that it changed.
    ///
    /// One thing these records cannot carry, and ADR-0316 moved which thing that
    /// is. A swap *in* is documented to start cold, so a replay meeting these
    /// records builds afresh and replays exactly. What no record says is that a
    /// slot was stopped: a version over the budget is recorded like any other,
    /// because it is what the slot holds, and a replay judges nothing — so it runs
    /// material the performance had frozen. That is a gap in the record vocabulary
    /// rather than in this function.
    fn record_procedure(&mut self, slot: usize, nodes: &Nodes) {
        if self.recorder.is_none() {
            return;
        }
        // One record per node, each at the address the watcher gave it — so a
        // slot with one renderer writes exactly the two lines it always did,
        // and a slot with a chain and two geometries writes a line for each.
        for (layer, index, hash) in nodes {
            let Some(record) = layer_named(layer).map(record_layer) else {
                eprintln!("  a node on layer `{layer}` is not in the record vocabulary");
                continue;
            };
            self.record_only(karakuri_store::record::Record::Procedure {
                slot: DeckSlot(slot as u8),
                at: karakuri_store::record::NodeAddress {
                    layer: record,
                    index: *index,
                },
                proc_hash: *hash,
            });
        }
    }

    /// Push a record without applying it.
    ///
    /// Two records are right here, and they are right for one reason: each
    /// *describes* a change that has already happened rather than asking for one. A
    /// `procedure` says what a slot became when a swap landed, and a `save` says a
    /// file exists — applying either would mean doing the thing a second time.
    /// Everything else goes through `Live::record`, which applies what it wrote.
    fn record_only(&mut self, record: karakuri_store::record::Record) {
        if let Some(recorder) = &mut self.recorder {
            recorder.push(record);
        }
    }

    /// Write what this run is playing as a Set file.
    ///
    /// The control that closes an open gap: `--save-set` writes what the *flags*
    /// say and exits, so the loop that lets an operator load a preset, edit it and
    /// watch it had no way to keep the result. This is the render-loop half of
    /// closing that — see
    /// `docs/adr/0122-a-save-writes-the-bytes-that-are-on-screen.md`. Every surface
    /// ends here — the key below, the MCP tool, and whatever surface arrives next —
    /// for the same reason every mix control ends in one method. This is the only
    /// save path, which is what makes a refusal and an outcome one sentence each
    /// rather than one sentence per surface.
    ///
    /// The slot is an argument and the key passes its focus in. A Set file
    /// describes one Set and a deck holds four; the one an *operator* means is the
    /// one their hands are already on, which is what focus is — and a model has no
    /// hands and no focus, so it names the slot as it names one to read a
    /// procedure. This is `toggle_on_air`'s split, for its reason.
    ///
    /// `id` is what the caller wanted it called, or a stamp. A key press cannot
    /// type a name, so it passes `None`; see `history::stamped_id`, whose
    /// convention that is and whose reason it borrows — an operator looks for the
    /// time they saved it. A caller that *can* type one is not made to take a
    /// timestamp.
    ///
    /// `reply` is whoever is waiting who is not at the terminal. Every sentence
    /// below goes to both, and each of them is written once: a refusal that reached
    /// a model in different words than it reaches the terminal would be two
    /// refusals to keep in step, and the wording of this one has already had to be
    /// corrected once.
    ///
    /// Read off the live Set, not off `Args`. `saving_seeds` states the hazard from
    /// the other side: a writer with its own copy of the rule records numbers the
    /// run was not using. Every number a Set file carries can have moved since the
    /// flags were parsed — a param through a record, a capacity or a salt through a
    /// rebuilt Set file — so the only reading that cannot be stale is the Set's
    /// own.
    ///
    /// Refused, accepted, gathered, written — and only the third of those needs a
    /// `Deck`. The three sentences a save can produce before the disk speaks are
    /// [`no_such_slot`], [`nothing_to_save`] and [`accepted_save`], all free
    /// functions, so what a client is told is checkable without a window.
    /// `playing_values` is the line that is not, and everything above it here is
    /// above it deliberately.
    ///
    /// Gathered here, written elsewhere. Everything below this line is a read off
    /// values already in memory; the store I/O goes to a thread of its own — one
    /// per save, since saves are rare and a pool would be machinery for a rate of a
    /// few an hour. The outcome comes back over `saves` and the record is written
    /// at the frame it arrives, not at this key press. See `Live::finished_saves`.
    fn save_set(
        &mut self,
        asked: Asked,
        slot: usize,
        id: Option<String>,
        reply: Option<mcp::Reply>,
    ) {
        // **Checked here rather than only where the request came from.** A key
        // press cannot name a slot this deck does not hold and a tool call can,
        // and below this line `playing_values` reads `deck.slot(slot)`, which
        // panics on one. The surface that asked refuses it too, in its own
        // words, so a model never reaches this — and this is the guard that
        // does not depend on it having.
        if !slot_in_range(slot, self.deck.slot_count()) {
            return refused(reply, no_such_slot(slot, self.deck.slot_count()));
        }
        let startup = self.startup.get(slot).map_or(&[][..], Vec::as_slice);
        let sources = live_sources(self.running.playing(slot), startup);
        if sources.is_empty() {
            return refused(
                reply,
                nothing_to_save(slot, self.loaded_set.as_deref(), startup.is_empty()),
            );
        }
        // **Named and answered above the GPU line**, which is where the whole
        // of [`accepted_save`]'s doc lives: `playing_values` below is the one
        // read here that needs a `Deck`, and the accept has to be on the side
        // of it a test can reach.
        let id = accepted_save(
            slot,
            asked,
            id,
            &sources,
            &self.store_root,
            reply
                .as_ref()
                .map(|r| r as &dyn karakuri_environment::SaveReply),
        );
        let values = playing_values(self.deck.slot(EngineSlot(slot as u8)).set(), &self.edges);
        let save = Save {
            slot,
            asked,
            id,
            root: self.store_root.clone(),
            sources,
            values,
        };
        let tx = self.save_tx.clone();
        // **A thread per save**, and detached: no frame waits for it. The *run*
        // waits, once, at the end and under a bound — see
        // [`Live::awaited_saves`], which is what this count is for.
        self.saves_in_flight += 1;
        std::thread::spawn(move || {
            let (slot, asked, id) = (save.slot, save.asked, save.id.clone());
            let outcome = save.run();
            // **Carried back rather than answered from here.** This thread
            // knows the outcome and could send it, and that would be a second
            // place a save is reported from: the record is written at the frame
            // the outcome lands, and a model told anything else than what that
            // frame decided would be reading a different story from the stream.
            let _ = tx.send(Saved {
                slot,
                asked,
                id,
                outcome,
                reply,
            });
        });
    }

    /// Every save that has landed since the last frame, said and recorded.
    ///
    /// Drained and never waited on: a frame owes the display a picture and owes a
    /// disk nothing.
    ///
    /// Called at the top of the frame, beside [`Live::run_requests`] and above
    /// `frame::compose` and everything downstream of it — not beside the swap-event
    /// drain, which is where it used to be, below the early returns a frame no
    /// longer has. See the comment at the head of [`Live::frame`]: a window that
    /// has faulted still has saves finishing behind it, and a run that told nobody
    /// about them until it quit was withholding the one answer a waiting client
    /// cannot get anywhere else. The swap drain stays below because a swap *is*
    /// about what was drawn; a save is not.
    ///
    /// The record is written here, at the frame the outcome arrived, which is the
    /// pattern `record_procedure` already follows — a record that describes a
    /// change already made. Writing one at the key press would be a stream claiming
    /// a file that the disk then refused, which is the failure this whole codebase
    /// is arranged against.
    fn finished_saves(&mut self) {
        let mut landed: Vec<Saved> = Vec::new();
        while let Ok(saved) = self.saves.try_recv() {
            landed.push(saved);
        }
        for saved in landed {
            self.took_save(saved);
        }
    }

    /// One save's outcome, said, recorded, and answered.
    ///
    /// One sentence for all three. What the terminal is told, what the stream
    /// records and what a waiting client is handed are the same fact, so the words
    /// are formed once here and the client gets the ones the operator got. The
    /// `Ok`/`Err` split is what a tool call's `isError` is built from — see
    /// [`mcp::Reply::settled`].
    fn took_save(&mut self, saved: Saved) {
        let Saved {
            slot,
            asked,
            id,
            outcome,
            reply,
        } = saved;
        self.saves_in_flight = self.saves_in_flight.saturating_sub(1);
        let said = match outcome {
            Ok(()) => {
                // **Two sentences because two things are true**, and the second
                // would be a lie in the first's words: `--load-set` reads the
                // library, so telling a model to load what it just wrote into
                // the sandbox would send it after a file that path cannot see.
                // What it is told instead is where the file is, which is what
                // the operator needs to find it after the show
                // (P-0096, ADR-0261).
                let said = match asked {
                    Asked::Operator => {
                        format!("slot {slot}: saved as set `{id}` — load it with `--load-set {id}`")
                    }
                    Asked::Model => format!(
                        "slot {slot}: saved as set `{id}` in the sandbox — \
                         `<store>/{}/{id}{}`. A save asked for over MCP is kept there \
                         rather than in the operator's library, so `--load-set {id}` does \
                         not reach it; the operator's own `k` writes the library",
                        karakuri_store::store::Store::SANDBOX,
                        karakuri_store::store::Store::SET_FILE_SUFFIX,
                    ),
                };
                eprintln!("{said}");
                self.record_only(karakuri_store::record::Record::Save {
                    slot: DeckSlot(slot as u8),
                    id,
                });
                Ok(said)
            }
            // **Printed, and nothing written.** See `Saved::outcome`.
            Err(e) => {
                let said = format!("slot {slot}: set `{id}` was not saved: {e}");
                eprintln!("{said}");
                Err(said)
            }
        };
        if let Some(reply) = reply {
            reply.settled(said);
        }
    }

    /// Every save still being written, waited for — up to [`SAVE_WAIT`].
    ///
    /// A frame owes the disk nothing, which is why [`finished_saves`] drains and
    /// never blocks. The end of the run is the one moment where that is the wrong
    /// trade: a save pressed in the last second reached the disk under an id
    /// nothing in the stream ever named, so the claim that a `save` record exists
    /// for every live save that reached the disk
    /// (`docs/adr/0120-a-record-may-reach-outside-the-stream.md`) was false in
    /// exactly the window an operator is most likely to be in — press `k`, see it
    /// took, quit.
    ///
    /// Bounded, because a disk can hang and quitting must not depend on one. The
    /// alternative was joining the threads, which is unbounded by construction: a
    /// store on a network mount that stops answering would take the window with it.
    /// Past the bound the run says how many saves it left behind and exits, which
    /// is the same trade the recorder makes when it counts the batches it lost
    /// rather than waiting for them.
    ///
    /// The wait is only ever paid by a run that pressed `k` and quit within a few
    /// frames; the count is zero for every other run and this returns without
    /// blocking.
    pub(crate) fn awaited_saves(&mut self) {
        self.finished_saves();
        if self.saves_in_flight == 0 {
            return;
        }
        eprintln!(
            "waiting up to {:.0}s for {} save{} still being written",
            SAVE_WAIT.as_secs_f32(),
            self.saves_in_flight,
            if self.saves_in_flight == 1 { "" } else { "s" }
        );
        let deadline = Instant::now() + SAVE_WAIT;
        for saved in drained_saves(&self.saves, self.saves_in_flight, deadline) {
            self.took_save(saved);
        }
        if self.saves_in_flight > 0 {
            eprintln!(
                "  {} save{} still unfinished after {:.0}s — each is written or it is not, \
                 and no record claims either way",
                self.saves_in_flight,
                if self.saves_in_flight == 1 { "" } else { "s" },
                SAVE_WAIT.as_secs_f32(),
            );
        }
    }

    /// One governor pass and what it decided, printed.
    ///
    /// Called when something the decision depends on moved — a residency request, a
    /// build landing — and never per frame: it allocates, and the answer cannot
    /// change between those events.
    ///
    /// The parked slots are named individually rather than counted, because each
    /// one is waiting on something different and the reasons call for different
    /// actions: `NoHeadroom` waits for a slot to come off air, `CommittedUnknown`
    /// for a measurement, and `NoPrimingNeeded` for nothing at all — that Set is
    /// closed form and can go straight on air.
    fn govern(&mut self, why: &str) {
        report_governing(&self.deck.govern(), why);
    }

    /// Cycle the focused slot's sync mode, skipping the modes its material cannot
    /// take and saying why.
    ///
    /// This is the CLI's form of a control greyed out: there is no widget to dim,
    /// so the unavailable modes are stepped over and the reason is printed with the
    /// result. Silently skipping would leave an operator pressing a key and
    /// watching two of three modes never arrive; printing on every press without
    /// skipping would make the key refuse to do anything at all on material that
    /// only allows one mode.
    ///
    /// Its record goes through [`Live::operate`] like every other settled key's,
    /// and it used to be built here. What kept it out was `Owed::NotSettled` on
    /// [`Operation::SetSync`]: `Transport::engaged` clamps the anchor against the
    /// session tempo, so whether the record carried the tempo that was asked for or
    /// the one the engine settled on read as a decision about the bytes on disk.
    /// The two are the same number — the clamp holds the anchor inside
    /// `karakuri_signal::oscillator::BPM_RANGE` and an oscillator's tempo is
    /// already inside it — so what was missing was never a decision but a reading,
    /// and `Current::tempo` is it. A cycle is still this surface's own:
    /// `Sync::ALL`, the skipping and the refusals are translations a keyboard
    /// makes, and what comes out of them is a destination (P-0090).
    fn cycle_sync(&mut self) {
        let slot = self.focus;
        let addr = EngineSlot(slot as u8);
        let current = self.deck.transport(addr).sync();
        let at = Sync::ALL.iter().position(|s| *s == current).unwrap_or(0);

        let mut refused: Vec<String> = Vec::new();
        let mut next = None;
        // Every other mode, in cycle order, starting after the current one.
        for step in 1..=Sync::ALL.len() {
            let candidate = Sync::ALL[(at + step) % Sync::ALL.len()];
            match self.deck.sync_allowed(addr, candidate) {
                Ok(()) => {
                    next = Some(candidate);
                    break;
                }
                Err(refusal) => refused.push(format!("{} — {refusal}", candidate.name())),
            }
        }

        let Some(next) = next else {
            // Unreachable while `Sync::Free` is never refused, and handled
            // rather than unwrapped because that is a property of `allows` and
            // not of this loop.
            eprintln!("slot {slot}: no sync mode is available for this material");
            return;
        };
        if next == current {
            eprintln!(
                "slot {slot}: {} is the only mode this material takes",
                next.name()
            );
        }
        for reason in &refused {
            eprintln!("  skipped {reason}");
        }

        // Read before the operation rather than after it, because `operate`
        // is about to anchor the slot at exactly this and the line below says
        // what the press did.
        let bpm = mix::current_tempo(self.deck.signals().oscillator());
        self.operate(&Operation::SetSync {
            deck: slot as u8,
            sync: mix::sync(next),
        });
        eprintln!(
            "slot {slot} sync {} at {bpm:.1} bpm{}",
            next.name(),
            match next {
                Sync::Free => " — wall time, the room does nothing to it",
                Sync::Tempo => " — 1x here, faster if the room is",
                Sync::Beat => " — locked to the room's position; u/i scrub",
            }
        );
    }

    /// Scrub the focused slot, in beats. The one control that goes backwards.
    fn scrub(&mut self, beats: f64) {
        let slot = self.focus;
        let addr = EngineSlot(slot as u8);
        let transport = self.deck.transport(addr);
        if transport.sync() != Sync::Beat {
            eprintln!(
                "slot {slot} is {} — scrubbing moves a position, and only beat sync has one (y)",
                transport.sync().name()
            );
            return;
        }
        self.operate(&Operation::ScrubDeck {
            deck: slot as u8,
            beats,
        });
        eprintln!(
            "slot {slot} scrub {:+.2} beats",
            self.deck.transport(addr).scrub_beats()
        );
    }

    /// A tap on the beat. Authoritative — a performer tapping is stating where the
    /// beat is, not offering evidence — and it goes onto the oscillator through the
    /// same `tempo` record a tracked correction does.
    ///
    /// Not through [`Live::operate`]. [`Operation::TapBeat`] is `Owed::NotSettled`
    /// — what a tap writes is the tracker's answer rather than a value, and the
    /// tracker may refuse — so this is also the one arm [`Live::run_surface`] keeps
    /// for itself. `b` and `note -> tap` are the same function for that reason, and
    /// both move the day the record is settled.
    fn tap(&mut self) {
        let started = self.started;
        let mut signals = *self.deck.signals();
        let Some(audio) = self.audio.as_mut() else {
            eprintln!("tap: no audio input — run with --audio-in to tap the beat");
            return;
        };
        let record = audio.tap(&mut signals, Instant::now(), started);
        self.deck.set_signals(signals);
        // **The record, or the tap did not happen as far as the stream is
        // concerned.** `Audio::tap` builds one and applies it; dropping it here
        // left a session whose grid had been moved by a hand with nothing in
        // the timeline to say so, and a replay then ran every `beats`-bound
        // parameter on a different phase. Found the day a control surface made
        // "every control writes a record" a claim rather than a habit.
        self.push_tempo(record);
        eprintln!(
            "tap: {:.1} bpm, phase set",
            self.deck.signals().oscillator().bpm()
        );
    }

    /// Halve or double the grid — the operator's last word on the octave.
    ///
    /// The tracker folds every candidate tempo into a one-octave window centred on
    /// the grid, so an octave error is stable rather than self-correcting: a set
    /// started at 87 for a track that is 174 will track 87 all night. This moves
    /// the grid and the window together, and the picture keeps its phase — doubling
    /// subdivides the beats already there.
    ///
    /// Not through [`Live::operate`], for [`Live::tap`]'s reason:
    /// [`Operation::ScaleGrid`] is `Owed::NotSettled` because moving the grid needs
    /// the beat tracker rather than a value, and the refusal below is the tracker's
    /// to give.
    fn shift_octave(&mut self, factor: f32) {
        let mut signals = *self.deck.signals();
        let Some(audio) = self.audio.as_mut() else {
            eprintln!("no audio input — the octave keys move the tracker's window, which only exists with --audio-in");
            return;
        };
        let before = signals.oscillator().bpm();
        match audio.octave(&mut signals, factor) {
            Some(record) => {
                self.deck.set_signals(signals);
                self.push_tempo(record);
                eprintln!(
                    "beat: grid {} to {:.1} bpm — the tracker's window moved with it",
                    if factor > 1.0 { "doubled" } else { "halved" },
                    self.deck.signals().oscillator().bpm()
                );
            }
            // Refused rather than applied and undone two seconds later: outside
            // the range the tracker searches there is nothing to lock to.
            None => eprintln!(
                "beat: {before:.1} bpm {} would leave the trackable range",
                if factor > 1.0 { "doubled" } else { "halved" }
            ),
        }
    }

    /// The offset for everything past the two outputs, which nothing here can
    /// measure. Found from where the audience stands, not from this machine — see
    /// `karakuri-audio`'s crate doc.
    fn nudge_latency_offset(&mut self, delta_ms: f32) {
        match self.audio.as_mut() {
            Some(audio) => {
                let ms = audio.nudge_latency_offset(delta_ms);
                // Which way it now points, said in words: the sign is the part
                // an operator gets wrong at 2 a.m., and "-15 ms" alone does not
                // say whether that is the picture waiting or the sound.
                let sense = if ms < 0.0 {
                    "the picture waits for the music"
                } else {
                    "the picture leads the music"
                };
                eprintln!("latency offset {ms:+.0} ms — {sense}");
            }
            None => eprintln!(
                "no audio input — the latency offset only means something with --audio-in"
            ),
        }
    }

    fn nudge_gain(&mut self, delta: f32) {
        self.set_gain(
            self.focus,
            self.deck.gain(EngineSlot(self.focus as u8)) + delta,
        );
    }

    fn set_gain(&mut self, slot: usize, gain: f32) {
        self.operate(&Operation::SetGain {
            deck: slot as u8,
            gain: clamp_gain(gain),
        });
        eprintln!(
            "slot {slot} gain {:.2}",
            self.deck.gain(EngineSlot(slot as u8))
        );
    }

    /// The fader, and the one control that silences a slot under every blend mode —
    /// which makes it the way out of material that has gone NaN. `[` and `]` move
    /// the *level*, and under `over` a level of zero is a black card that still
    /// covers what is beneath it.
    ///
    /// The mode is printed with the number because what the number does depends on
    /// it: under `add` opacity and gain are the same dial twice.
    fn nudge_opacity(&mut self, delta: f32) {
        let slot = self.focus;
        self.set_opacity(slot, self.deck.opacity(EngineSlot(slot as u8)) + delta);
    }

    fn set_opacity(&mut self, slot: usize, value: f32) {
        let opacity = value.clamp(0.0, 1.0);
        self.operate(&Operation::SetOpacity {
            deck: slot as u8,
            opacity,
        });
        let addr = EngineSlot(slot as u8);
        eprintln!(
            "slot {slot} opacity {:.2} ({})",
            self.deck.opacity(addr),
            self.deck.blend(addr).name()
        );
    }

    /// Fade the focused slot's fader to `to`, over the current length, starting on
    /// the current quantum.
    ///
    /// Opacity rather than gain, because opacity is the fader: it silences a slot
    /// under every blend mode, where a gain of zero under `over` is a black card
    /// that still covers. A gain fade is reachable through the record and
    /// deliberately has no key — two keys that look alike and differ only under one
    /// blend mode is how an operator ends up fading the wrong one in the dark.
    /// `Operation::FadeDeck` says the same thing at its own definition, which is
    /// why the choice is not repeated in the record this writes.
    ///
    /// One `operate` call, and [`Live::fade_slot`] is gone. This built its record
    /// where it stood while the quantum and the length had no owner; they are the
    /// surface's, they are handed over as
    /// `karakuri_operation_record::Current::transition`, and what this key writes
    /// is `written`'s answer like every other settled key's.
    fn fade(&mut self, to: f32) {
        let slot = self.focus;
        self.operate(&Operation::FadeDeck {
            deck: slot as u8,
            to,
        });
        eprintln!(
            "slot {slot} fading to {to:.2} over {} beat{} from {}",
            self.fade_beats,
            if self.fade_beats == 1.0 { "" } else { "s" },
            self.quantum_name()
        );
    }

    /// A crossfade: the focused slot out and the next one in, together.
    ///
    /// Two scheduled moves rather than a `Crossfade` object, which is the whole
    /// argument of `karakuri_engine::transition` seen from the keyboard — the
    /// first-class thing is the move, and every gesture anyone names is made of
    /// those. They share a start and a length, so they are one gesture without
    /// being one type.
    ///
    /// The incoming slot is put at silence and then on air, in that order, and both
    /// halves of that are load-bearing. A slot comes up at full opacity and going
    /// off air does not lower it, so putting one on air without silencing it first
    /// shows it at full immediately — up to a bar before the fade it is supposed to
    /// arrive on, which is a cut with a decorative fade attached. And a fade to
    /// something that is not being composited is a fade to black, so it does have
    /// to go on air.
    ///
    /// The silencing is a `opacity` record like any other, so it cancels nothing
    /// the operator wanted and replays like anything else.
    ///
    /// All four of its records are one operation now, and this function is one
    /// `operate` call — which is the sentence that used to be here as a promise. It
    /// read *"the day that is settled this function is one `operate` call"*, and
    /// the day was the one the quantum and the length got an owner: they are the
    /// surface's, `karakuri_operation_record::Current::transition` is where this
    /// program hands them over, and `written` answers `Operation::Crossfade` with
    /// the four records in the order above.
    ///
    /// The two remaining lines are the keyboard's translation and not the gesture.
    /// *The next deck* is what `x` means here and the operation names both decks,
    /// so working out which one and refusing a deck with nowhere to go stays;
    /// everything past that is the conversion's.
    ///
    /// The put-on-air is written even where the slot is already live, which is the
    /// one thing this changed about the stream. It used to be conditional on
    /// `Deck::residency`, and the conversion has no deck to ask:
    /// `Operation::Crossfade` says four records at its own definition, and a
    /// `residency` record for a slot that is already live decodes to a state it is
    /// already in.
    fn crossfade(&mut self) {
        let from = self.focus;
        let to = (from + 1) % self.deck.slot_count();
        if to == from {
            eprintln!("crossfade needs somewhere to go — this deck holds one slot");
            return;
        }
        self.operate(&Operation::Crossfade {
            from: from as u8,
            to: to as u8,
        });
        eprintln!(
            "crossfade {from} to {to} over {} beat{} from {}",
            self.fade_beats,
            if self.fade_beats == 1.0 { "" } else { "s" },
            self.quantum_name()
        );
    }

    /// Wipe the next slot in over the focused one.
    ///
    /// A mask and one scheduled move, and that is the whole of it: the incoming
    /// slot is given the current shape at position 0 — revealing nothing — put on
    /// air under `over` so that what it reveals *hides* what is beneath, and then
    /// one transition carries the front from 0 to 1. Nothing in the transition
    /// system knows what a mask is and nothing in the mask knows what a beat is.
    ///
    /// All six of its records are one operation now, and this function is one
    /// `operate` call — six where nothing is already where the wipe is putting it,
    /// and four or five where something is — which is [`Live::crossfade`]'s
    /// paragraph one gesture along and the last of them to be written. It built
    /// five of the six out of five separate operations and the sixth by hand, while
    /// what a wipe owed had no owner: the *shape* its front takes and the soft
    /// edge. Both have one. The shape is `Operation::SetTransition`'s third setting
    /// — the one this program holds in `mask_kind` and `mask_angle` and the `z` key
    /// writes — so it goes over with the quantum and the length inside
    /// `karakuri_operation_record::Current::transition`, which is where its two
    /// neighbours already were. The soft edge is read off the mask on the deck
    /// being wiped in, which is `mix::current_mask` and was already the reading
    /// `Operation::SetMaskShape` takes.
    ///
    /// The two lines that are left are the keyboard's translation and not the
    /// gesture. *The next slot* is what `c` means here and the operation names both
    /// decks, so working out which one and refusing a deck with nowhere to go
    /// stays. So does the refusal with no shape chosen: `Operation::Wipe` says
    /// *"Refused with no shape chosen"* at its own definition, `written` has no
    /// answer that is a refusal, and the shape is this surface's own setting — so
    /// this is the only place that can turn a wipe with nothing to move away, and
    /// it does it before it asks.
    ///
    /// The one decision this function used to make is made in the conversion now,
    /// and it is not a record. Under `add` the same gesture is a wipe *on* rather
    /// than a wipe *over*, which is a different picture and a legitimate one — so
    /// the mode is left wherever the operator had it and `over` is written only
    /// where the slot is still at the mode a slot starts in, which is what makes
    /// `m` in front of `c` mean something. The put-on-air is written only where the
    /// slot is not already live, on the same terms. Those were two `if`s here and
    /// they are the `Wipe` arm's now, because the sentence belongs beside the
    /// records it governs rather than on one of the surfaces that can reach them
    /// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// — a rule held in one surface binds none of the other three). What the
    /// conversion needed to keep it is a reading, which is the third this gesture
    /// takes: `karakuri_operation_record::Current::mix`, handed over by
    /// [`Live::operate`] out of the deck this function no longer touches.
    ///
    /// Six records where it once wrote five, and the extra one is what routing the
    /// mask honestly costs rather than an accident: the shape and the front are two
    /// operations
    /// (`docs/adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md`)
    /// and each of them writes a whole `Record::Mask`, because the record is a
    /// state and not an ask. The pair lands in that order, so the front is at 0
    /// when the move is scheduled — which is the same picture the one hand-built
    /// record made.
    fn wipe(&mut self) {
        let under = self.focus;
        let over = (under + 1) % self.deck.slot_count();
        if over == under {
            eprintln!("a wipe needs somewhere to come from — this deck holds one slot");
            return;
        }
        if self.mask_kind == MaskKind::None {
            eprintln!("no mask shape — `z` chooses one, and a wipe is a shape moving");
            return;
        }
        self.operate(&Operation::Wipe {
            from: under as u8,
            to: over as u8,
        });
        eprintln!(
            "wipe {over} over {under} — {} at {:.0}°, over {} beat{} from {}",
            self.mask_kind.name(),
            self.mask_angle.to_degrees(),
            self.fade_beats,
            if self.fade_beats == 1.0 { "" } else { "s" },
            self.quantum_name()
        );
    }

    /// The shape the next wipe uses, and which way it runs.
    ///
    /// One key for both, because the shapes and the angles an operator actually
    /// reaches for are a short list rather than two dials: across, up, the two
    /// diagonals, and an iris. A continuous dial for the angle is not built: there
    /// is nowhere on this surface to see one.
    fn cycle_mask(&mut self) {
        let at = MASK_SHAPES
            .iter()
            .position(|(k, a, _)| *k == self.mask_kind && *a == self.mask_angle)
            .unwrap_or(0);
        let (kind, angle, name) = MASK_SHAPES[(at + 1) % MASK_SHAPES.len()];
        self.mask_kind = kind;
        self.mask_angle = angle;
        eprintln!("wipes are {name}");
    }

    /// Choose which renderer of the focused slot is the live one, on the grid.
    ///
    /// The first half of a variant pool, and the half that is true today: several
    /// renderers over *one* simulation, in one Set, one of them folded into the
    /// picture at a time. A pool spanning deck slots that differ at a layer slot
    /// was the alternative and is rejected — see
    /// `docs/adr/0148-a-variant-pool-is-a-set-and-the-deck-stays-a-mixer.md` — and
    /// the rest of it — an alternative that differs at L1, priming it off air,
    /// sharing the geometry between them — is not built. See the manual, "Selecting
    /// one renderer of a slot".
    ///
    /// What it does not save. The renderers that are not selected go on drawing,
    /// each into a target of its own: at 1280x720 that is 7.03 MB and a render pass
    /// apiece, every frame, whether or not anybody is looking at them. That is what
    /// makes the selection a uniform write and a cut on the beat rather than a
    /// build — cheap, and not free. The line printed says how many are still
    /// drawing for exactly that reason.
    ///
    /// Cycling from the selection that is armed, not from the one on screen. A
    /// press lands up to a bar later, so two presses inside a bar read the same
    /// live edge and would both choose the same renderer — the second press would
    /// do nothing and say it had. `Deck::selections_on` is what makes the second
    /// press mean "the one after that".
    ///
    /// One way, and it is stated rather than discovered: there is no position in
    /// the cycle that puts every renderer back. A Set comes up with all of them
    /// folded — or folded to the one its Set file's `merge` record named, which is
    /// the other way to start — and the first press leaves that state for good,
    /// which is what "makes one live and the rest not" costs when the record names
    /// one renderer. Restoring the fold is a different statement and wants its own
    /// vocabulary — an `Option` carrying `null` for "all of them" is the shape it
    /// would take, which is the shape the retired `preview` record had.
    ///
    /// What it chooses is kept. A live save reads the fold off the Set — see
    /// `playing_values` — so `k` after a press writes `{"t":"merge", "live":N}` and
    /// loading that file back comes up on renderer N.
    ///
    /// Its record comes out of [`Live::operate`], which it did not while the
    /// instant a selection lands on had nobody to supply it: the quantum is this
    /// program's setting, `mix::current_transition` is where it becomes an instant,
    /// and `written` writes the `select` record from it. Which renderer to move to
    /// is the keyboard's own translation and stays here, which is the whole of what
    /// is left of this function's arithmetic.
    fn cycle_renderer(&mut self) {
        let slot = self.focus;
        let addr = EngineSlot(slot as u8);
        let set = self.deck.slot(addr).set();
        let count = set.inputs().len();
        let composited = set.layering() == karakuri_engine::set::Layering::Composite;
        // The live edges, read before anything is scheduled: with nothing
        // selected yet every one of them is live, which is the state a Set
        // builds in.
        let live: Vec<usize> = set
            .inputs()
            .iter()
            .enumerate()
            .filter(|(_, input)| input.live)
            .map(|(at, _)| at)
            .collect();
        if count < 2 {
            eprintln!(
                "slot {slot} draws with one renderer — there is nothing to choose between. \
                 A second `.kir` on the `--set` for this slot is what makes a choice"
            );
            return;
        }
        if !composited {
            eprintln!(
                "slot {slot} overdraws its {count} renderers — they share one target and \
                 meet through their own blend states, so there is no edge to silence. \
                 `--merge {slot}` gives each a target of its own"
            );
            return;
        }
        let armed = self.deck.selections_on(addr).next().map(|s| s.renderer());
        let next = match (armed, live.as_slice()) {
            // The one waiting to land, so a second press inside the same bar
            // moves past it rather than choosing it again.
            (Some(at), _) => (at + 1) % count,
            // Exactly one live is a selection that has landed.
            (None, [at]) => (at + 1) % count,
            // Every renderer live, which is what a Set comes up as, or none of
            // them, which nothing here produces: start at the first.
            (None, _) => 0,
        };
        self.operate(&Operation::SelectRenderer {
            deck: slot as u8,
            renderer: next as u32,
        });
        eprintln!(
            "slot {slot} renderer {next} of {count} from {} — {}",
            self.quantum_name(),
            // The cost, said on every press rather than in the manual alone: a
            // selection that reads as free is one an operator will reach for
            // where a rebuild was wanted.
            if count == 2 {
                "the other one still draws into a target of its own".to_string()
            } else {
                format!(
                    "the other {} still draw into targets of their own",
                    count - 1
                )
            }
        );
    }

    /// Where a scheduled move starts: now, the next beat, or the next bar.
    fn cycle_quantum(&mut self) {
        let at = QUANTA
            .iter()
            .position(|(q, _)| *q == self.quantum)
            .unwrap_or(0);
        self.quantum = QUANTA[(at + 1) % QUANTA.len()].0;
        eprintln!("fades start {}", self.quantum_name());
    }

    /// How long a scheduled move lasts.
    fn cycle_fade_beats(&mut self) {
        let at = FADE_BEATS
            .iter()
            .position(|b| *b == self.fade_beats)
            .unwrap_or(0);
        self.fade_beats = FADE_BEATS[(at + 1) % FADE_BEATS.len()];
        eprintln!(
            "fades last {} beat{}",
            self.fade_beats,
            if self.fade_beats == 1.0 { "" } else { "s" }
        );
    }

    fn quantum_name(&self) -> &'static str {
        QUANTA
            .iter()
            .find(|(q, _)| *q == self.quantum)
            .map(|(_, name)| *name)
            .unwrap_or("now")
    }

    /// Cycle the focused slot's blend mode. No refusals here — unlike sync, every
    /// mode is available to every slot, because a blend mode is a question about
    /// pixels and not about what the material can do.
    fn cycle_blend(&mut self, slot: usize) {
        let addr = EngineSlot(slot as u8);
        let current = self.deck.blend(addr);
        let at = Blend::ALL.iter().position(|b| *b == current).unwrap_or(0);
        let next = Blend::ALL[(at + 1) % Blend::ALL.len()];
        self.operate(&Operation::SetBlendMode {
            deck: slot as u8,
            blend: mix::blend_mode(next),
        });
        eprintln!(
            "slot {slot} blend {} — gain {:.2}, opacity {:.2}",
            self.deck.blend(addr).name(),
            self.deck.gain(addr),
            self.deck.opacity(addr)
        );
    }

    fn cycle_tonemap(&mut self) {
        self.operate(&Operation::SetTonemap {
            tonemap: mix::tonemap(next_tonemap(self.look.op)),
        });
        eprintln!(
            "tonemap {} (exposure {:.2})",
            op_name(self.look.op),
            self.look.exposure
        );
    }

    fn set_exposure(&mut self, exposure: f32) {
        self.operate(&Operation::SetExposure {
            exposure: clamp_exposure(exposure),
        });
        eprintln!(
            "exposure {:.3} ({})",
            self.look.exposure,
            op_name(self.look.op)
        );
    }

    /// Every mix change goes through here, and here goes through a record.
    ///
    /// Built, decoded, and only then applied — so what drives the deck is what a
    /// replay would decode from a session stream, rather than a second path that
    /// happens to agree with it today. `karakuri-environment`'s `audio.rs` does the
    /// same thing with the two records it emits; see the program's `mix.rs` for the
    /// whole argument.
    ///
    /// A record this build cannot obey is printed and nothing moves. It cannot
    /// happen from a key press — every caller here built the record a moment ago
    /// out of the engine's own types — and it is handled rather than unwrapped
    /// because the replay driver will hand this same function lines off a file, and
    /// a file is where an unobeyable record comes from. A tempo correction into the
    /// stream.
    ///
    /// Separate from [`Live::record`] because a `tempo` is applied where it is
    /// decided rather than read back — the oscillator is moved by the code that
    /// worked out how far, and `apply_replayed` is what re-applies it on the way
    /// back. What this owes is the *writing*, and it is one function so that a
    /// third thing moving the grid cannot forget it: two already had.
    ///
    /// Scalars only, so pushing it allocates nothing, which is what lets the frame
    /// path call it as well as the two keys.
    fn push_tempo(&mut self, record: karakuri_store::record::Record) {
        if let Some(recorder) = &mut self.recorder {
            recorder.push(record);
        }
    }

    /// A surface's operation, as the records it writes — and then written.
    ///
    /// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// puts every control at the same record, and this is where a key press ends:
    /// the operation is named, `karakuri-operation-record` says what it writes, and
    /// [`Live::record`] writes it and reads it back the way every other record here
    /// is read back. A console fader and a mapped MIDI control are the same thing
    /// exactly because they arrive at this function carrying the same name.
    ///
    /// The reading is taken here and nowhere else. The conversion is not pure —
    /// `SetExposure` becomes a `look` record carrying the operator and the white
    /// point too — so what is running has to be read back and handed over, and this
    /// is the only place in this program that knows both what was asked for and
    /// what is on screen.
    ///
    /// The two answers that are not records are printed rather than swallowed,
    /// which is what this wrapper is: a key press and a mapped control have nobody
    /// waiting on an answer, and an operator pressing a key that does nothing
    /// deserves the sentence.
    ///
    /// A model does have somebody waiting, so [`Live::run_operations`] calls
    /// [`Live::performed`] instead and hands the same sentence back as the call's
    /// error. The words are one string built in one place ([`answered`]), which is
    /// [`refused`]'s rule: a copy of them for the second audience is free to be
    /// right on the day it is written and wrong at the next correction.
    fn operate(&mut self, operation: &Operation) {
        if let Err(said) = self.performed(operation) {
            eprintln!("{said}");
        }
    }

    /// [`Live::operate`], with the answer handed back rather than printed.
    ///
    /// `Ok` means the records were written and read back, which is the whole of
    /// what performing an operation is on this surface. `Err` is the sentence
    /// [`answered`] built, and it means nothing on this run changed — see there for
    /// why that is a refusal rather than a line on a terminal somebody may not be
    /// reading.
    fn performed(&mut self, operation: &Operation) -> Result<(), String> {
        let transport = match operation {
            Operation::ScrubDeck { deck, .. } => EngineSlot::new(*deck, self.deck.slot_count())
                .map(|slot| mix::current_transport(self.deck.transport(slot))),
            _ => None,
        };
        // The mask of the deck the operation names, on the transport's terms:
        // both halves of a mask write the record whole, so each of them needs
        // the half it did not ask for.
        // A wipe names two decks and is read for one of them: everything it
        // writes is about the deck arriving, so the mask handed over is
        // `to`'s. The soft edge is the only field of it a wipe does not say,
        // and giving it the covered deck's would put somebody else's edge on
        // the front that is about to cross the frame.
        let mask = match operation {
            Operation::SetMaskShape { deck, .. }
            | Operation::SetMaskPosition { deck, .. }
            | Operation::Wipe { to: deck, .. } => EngineSlot::new(*deck, self.deck.slot_count())
                .map(|slot| mix::current_mask(self.deck.mask(slot))),
            _ => None,
        };
        // **The session tempo, for the one operation anchored to it.**
        // Engaging a mode anchors the slot at the tempo the room is going at,
        // and this reading is what `written` builds `Record::Transport` from —
        // `mix::current_tempo` takes the oscillator rather than a number, so
        // nothing here can hand in a tempo the session never ran at.
        let tempo = match operation {
            Operation::SetSync { .. } => Some(mix::current_tempo(self.deck.signals().oscillator())),
            _ => None,
        };
        // **The transition settings, for the four operations that schedule a
        // move**, and the one reading here that is read off nothing: `z`, `n`
        // and `j` set the shape, the quantum and the length, no record carries
        // any of them, and they are this program's own state until it hands
        // them over. That is the whole of what settling these four
        // conversions decided — see `karakuri_operation_record::Transition`.
        //
        // **The shape is the third setting and the wipe is the only reader.**
        // `mask_kind` and `mask_angle` are what `z` cycles, they are the shape
        // the *next* wipe takes rather than the shape any slot is wearing, and
        // handing them over here is what stopped `Live::wipe` building a
        // record of its own.
        //
        // `mix::current_transition` takes the oscillator and the quantum
        // rather than an instant, so the start is
        // `karakuri_engine::transition::quantise`'s answer and this file
        // cannot hand in a beat the grid was never on.
        // **Where the deck a wipe is arriving on already sits in the mix**,
        // and the one reading here taken so that a record can be left *out*.
        // A wipe puts that deck under `over` and on air; this program is what
        // knows the deck is already there, and the conversion is where the two
        // records that would say so again are dropped. `c` used to make that
        // decision here, in the two `if`s that are gone — the mode is the
        // operator's, and `m` in front of `c` is what it buys.
        //
        // **The deck as it reports, which is what the governor may have held
        // below the request.** `mix::current_mix` takes the engine's two
        // values rather than the vocabulary's, so the crossing is made in one
        // place, exactly as the mask's and the transition's are.
        let mix = match operation {
            Operation::Wipe { to: deck, .. } => EngineSlot::new(*deck, self.deck.slot_count())
                .map(|slot| mix::current_mix(self.deck.blend(slot), self.deck.residency(slot))),
            _ => None,
        };
        let transition = match operation {
            Operation::FadeDeck { .. }
            | Operation::Crossfade { .. }
            | Operation::SelectRenderer { .. }
            | Operation::Wipe { .. } => Some(mix::current_transition(
                self.deck.signals().oscillator(),
                self.quantum,
                self.fade_beats,
                mix::FADE_CURVE,
                self.mask_kind,
                self.mask_angle,
            )),
            _ => None,
        };
        let current = Current {
            look: Some(mix::current_look(&self.look)),
            // **The chain that is running, read off the `Present` that holds
            // it** — `Engine::look`'s seam one pass along, and unconditional
            // for the look's reason: this surface has one of each and asking
            // which operation wants which would be a second list to keep in
            // step with `written`'s.
            //
            // **No key reaches the three master rows on this surface**, so
            // nothing here writes one today; it is handed in all the same,
            // because what decides whether a chain operation can be answered
            // is whether the reading was taken and not which surface asked
            // (ADR-0317, and `written`'s `Owed::NotRead`).
            master_chain: Some(mix::current_chain(&self.present.chain_spec())),
            transport,
            mask,
            tempo,
            transition,
            mix,
        };
        for record in answered(operation, &current)? {
            self.record(record);
        }
        Ok(())
    }

    pub(crate) fn record(&mut self, record: karakuri_store::record::Record) {
        if let Some(recorder) = &mut self.recorder {
            // Cloned, which allocates for the records that carry a name —
            // `look`, `mask`, `blend`, `residency`, `transport` — and for no
            // other. This was written when the only way in was a key press. A
            // mapped MIDI fader reaches it from `Live::run_surface` inside
            // `Live::frame`, so on `exposure` and `mask-position` it was a
            // small heap touch per control-change message on the render
            // thread, which is the first rule this repository has. It is now
            // **once a frame per control**: `crate::midi`'s router keeps the
            // last value a continuous control sent within a frame and drops
            // the ones before it, so a sweep of several hundred messages
            // reaches here once
            // (`docs/adr/0207-a-continuous-control-says-one-thing-per-frame.md`;
            // the program's `mix.rs` carries the measurement, which is why the
            // coalescer is
            // there).
            recorder.push(record.clone());
        }
        match mix::change(&record, self.deck.slot_count()) {
            Ok(Some(change)) => self.apply(change),
            Ok(None) => {}
            Err(message) => eprintln!("mix: {message}"),
        }
    }

    /// What one decoded record does. The only place the mix is written.
    fn apply(&mut self, change: mix::Change) {
        match change {
            mix::Change::Gain { slot, value } => self.deck.set_gain(EngineSlot(slot as u8), value),
            mix::Change::Opacity { slot, value } => {
                self.deck.set_opacity(EngineSlot(slot as u8), value)
            }
            mix::Change::Blend { slot, mode } => self.deck.set_blend(EngineSlot(slot as u8), mode),
            mix::Change::Mask { slot, mask } => self.deck.set_mask(EngineSlot(slot as u8), mask),
            mix::Change::Transition {
                slot,
                control,
                to,
                start,
                beats,
                curve,
            } => {
                let t = schedule_from(&self.deck, slot, control, to, start, beats, curve);
                self.deck.schedule(t);
            }
            // **Checked against the Set on screen**, which is the only place
            // the answer is: the decoder knows the deck's size and not what is
            // in it. A record from a session recorded against a Set with three
            // renderers and replayed against one with two lands here.
            mix::Change::Select {
                slot,
                renderer,
                start,
            } => {
                let count = self.deck.slot(EngineSlot(slot as u8)).set().inputs().len();
                match renderer_in_range(slot, renderer, count) {
                    Ok(()) => {
                        self.deck
                            .schedule_selection(karakuri_engine::transition::Selection::new(
                                slot, renderer, start,
                            ))
                    }
                    Err(refusal) => eprintln!("{refusal}"),
                }
            }
            // **The one change that reaches inside a Set.** Every arm around
            // it moves the deck the Sets are playing on; this writes a number
            // into the Set in one slot, and `Deck::write_param` is the public
            // road to it. It compiles nothing — the value is packed into the
            // uniform by the next `Set::prepare`, which is the next frame.
            //
            // **A rebuild does not carry it**, and that is not settled here:
            // what a `--watch` rebuild restates is `swap::Request::params`, and
            // nothing puts a live write there. See
            // `docs/adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md`,
            // which names it as the open question and why it is a bigger one
            // than the record was.
            mix::Change::Ride { slot, writes } => {
                for write in &writes {
                    match self.deck.write_param(EngineSlot(slot as u8), write) {
                        Ok(0) => eprintln!("{}", no_such_param(slot, &write.key)),
                        Ok(_) => {}
                        Err(refused) => eprintln!("{refused}"),
                    }
                }
            }
            // **The other change that reaches inside a Set**, beside the
            // ride above: this one says what a parameter blends *towards*
            // rather than what it blends *from*. `Deck::bind` and
            // `Deck::unbind` are the public roads, and neither compiles
            // anything — a binding is resolved by the next `Set::prepare`.
            //
            // **A rebuild does not carry it**, on the ride's own terms and for
            // the same reason: `swap::Request::bindings` is restated from
            // `Watch::bindings`, which only a re-point writes. See
            // `docs/adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md`.
            mix::Change::Source {
                slot,
                layer,
                index,
                key,
                binding,
            } => match binding {
                Some(binding) => match self.deck.bind(EngineSlot(slot as u8), binding) {
                    karakuri_engine::set::Bound::Yes => {}
                    karakuri_engine::set::Bound::NoSuchParam => {
                        eprintln!("{}", no_such_param(slot, &key))
                    }
                    karakuri_engine::set::Bound::NoSuchControl => eprintln!(
                        "slot {slot}: `{key}` is bound to a control this Set does not publish"
                    ),
                },
                None => {
                    if !self.deck.unbind(EngineSlot(slot as u8), layer, index, &key) {
                        eprintln!("slot {slot}: nothing was driving `{key}`");
                    }
                }
            },
            // **Who may move one node**, and it changes what one knob may do
            // from the next press: a bare-name write over nodes that are not
            // all under one authority is refused whole by `Set::write_param`.
            mix::Change::Authority {
                slot,
                layer,
                index,
                authority,
            } => {
                if !self
                    .deck
                    .set_authority(EngineSlot(slot as u8), layer, index, authority)
                {
                    eprintln!(
                        "slot {slot}: no node {}:{index} to make {}",
                        karakuri_environment::meta::layer_name(
                            karakuri_environment::meta::layer_of(layer),
                        ),
                        authority.name()
                    );
                }
            }
            mix::Change::Residency { slot, level } => {
                self.deck.set_residency(EngineSlot(slot as u8), level);
                // A slot arriving or leaving changes what is committed, and the
                // deck's headroom with it: a slot that could not be admitted a
                // moment ago may fit now, and one that fitted may not. Requests
                // are untouched by the pass, so a park recovers on its own the
                // moment there is room.
                self.govern("residency");
            }
            // **Stored and not applied.** `frame::compose` writes the tone map
            // uniform every frame from the look the committing closure hands
            // it, so writing it here as well would be a second writer of one
            // value — the shape this whole module set out to remove, in
            // miniature. `Live::apply_look` is gone with it.
            mix::Change::Look(look) => self.look = look,
            // **The level at the chain's entry, applied where the mix writes
            // the composited frame** — `Deck::set_out`, which names no slot
            // because it acts on what the fold produced (ADR-0224).
            mix::Change::MasterOut(value) => self.deck.set_out(value),
            // **And the chain itself, applied and not stored, which is the
            // opposite of the look one arm up and for a stated reason.** The
            // `Present` is where a chain lives — it owns the targets its slots
            // read and write — so there is nowhere else to put it, and a copy
            // on this struct beside it would be the second writer the look arm
            // refuses (ADR-0317, and `Present::chain_spec` is how it is read
            // back).
            //
            // **What it costs is now two things and the record decides
            // which**: a list whose shape is the shape already running is a
            // uniform write per slot, and any other is a build. See
            // `mix::apply_chain`.
            //
            // **A refusal is said and the chain that is running stays.** An
            // address the store does not hold is a record this build cannot
            // obey, which is reported at the operation rather than drawn as a
            // wrong picture.
            mix::Change::MasterChain(slots) => {
                if let Err(refusal) = mix::apply_chain(
                    &mut self.present,
                    &self.gpu.device,
                    &self.gpu.queue,
                    &slots,
                    // **The shipped three and nothing else, and the refusal
                    // says which address it could not find.** This surface has
                    // no key that names a slot of the chain — `written` is
                    // handed the reading all the same, for its own reason — so
                    // the only lists it can be handed are ones made of the
                    // presets. An address out of a store is what M5.16's second
                    // pass owes, with the Library's drop; opening one here
                    // would create a store a plain run never asked for.
                    &|address| mix::resolve_procedure(None, address),
                ) {
                    eprintln!("  {refusal} — the chain keeps what it had");
                }
            }
            mix::Change::Transport {
                slot,
                sync,
                anchor_bpm,
                scrub_beats,
            } => {
                // The refusal is reported and nothing moves. It cannot happen
                // from a key press — `cycle_sync` only offers modes the Set
                // allows — but a session recorded against one Set and replayed
                // against another is exactly where it can, and a slot silently
                // left free would be a performance replayed wrong.
                if let Err(refusal) =
                    self.deck
                        .set_transport(EngineSlot(slot as u8), sync, anchor_bpm, scrub_beats)
                {
                    eprintln!("slot {slot}: {} sync refused — {refusal}", sync.name());
                }
            }
        }
    }

    pub(crate) fn frame(&mut self) {
        self.run_demo();
        self.run_surface();
        // **Both halves of the save path, and both above the GPU work below.**
        // Taking the request here is [`Live::run_requests`]'s own reasoning: a
        // client asking to keep what is playing should not be waiting on a
        // swapchain, and nothing a request reaches needs the GPU. The drain
        // used to sit under `frame::compose`, which implemented half of that
        // and quietly withheld the other: a window latched to `Skip::Fault`, or
        // returning `Outdated` every frame, returned before it — so the request
        // was taken, the save thread wrote the file, and the
        // client waited out `SAVE_REPLY` to be told the outcome was neither
        // success nor failure about a save that had already landed. The
        // terminal never said "saved as set X" either, and the `save` record
        // was withheld from the stream until the run quit. A save's outcome has
        // nothing to do with whether there is a surface to draw on, which is
        // exactly what the two calls being here rather than there says.
        self.run_requests();
        self.finished_saves();

        // **The ordering that used to be a comment is still the shape of the
        // call, and what it orders has changed.** A `tick` is a promise that
        // the deck advanced by that many steps, and what makes it honest is
        // that `frame::compose` calls the closure below and renders the deck
        // with nothing between them. It is no longer that a frame with nowhere
        // to draw records nothing: it is that there is no such frame. The
        // defect this all came from is unchanged and is
        // `docs/adr/0078-a-frame-that-is-discarded-must-not-already-have-been-recorded.md`
        // — the loop read the clock, wrote the `tick`, measured the audio, and
        // *then* found the swapchain had nothing, and `Outdated` arrives on
        // every resize, so resizing during a recorded session made the replay
        // diverge from the performance.
        //
        // **The window is one sink and does not gate the frame.** A frame it
        // refuses is composed anyway — the clock is read, the audio is
        // measured, the `tick` is written, the deck advances — and the only
        // thing the refusal costs is the picture, which is what an output being
        // off has to mean before there is a second one. So `Clock::last` now
        // moves on every frame this function runs and no interval is carried
        // between frames: the gap that still has to survive is the one where
        // this function does not run at all, and `MAX_STEPS` is the anti-spiral
        // clamp on it however long it was.
        let Live {
            gpu,
            deck,
            present,
            sink,
            clock,
            audio,
            recorder,
            look,
            tempo_source,
            ..
        } = self;
        // One sink today, and the slice is the whole of what a second one — a
        // projector, a plugin — costs this function. See `docs/plugins.md`.
        let mut sinks: [&mut dyn frame::Sink; 1] = [sink];
        let outcome = frame::compose(
            gpu,
            deck,
            present,
            &mut sinks,
            // A `Fault` is printed unconditionally, because it is already at
            // most one per condition: the sink latches it — see `frame::Skip`.
            // The latch used to be here, which meant the message was *built*
            // every frame and thrown away, an allocation on the frame path for
            // as long as the window stayed broken.
            //
            // `Transient` is the swapchain being remade and the next frame
            // asking again; there is nothing to say about it sixty times a
            // second. The index says which sink refused, and there is one, so
            // nothing here reads it yet.
            &mut |_at, skip| {
                if let frame::Skip::Fault(why) = skip {
                    eprintln!("surface: {why}");
                }
            },
            |deck| {
                let steps = clock.steps(Instant::now());
                // **The source first, and it takes the grid with it.** Both end in
                // a `tempo` record and `Oscillator::correct` is last-writer-wins,
                // so running the source first and letting the tracker follow would
                // have meant the tracker winning — it returns a trim on every
                // frame once locked, against the source's four a second. The order
                // is not what settles it: `Grid::Followed` is, by telling the
                // tracker to keep tracking and keep quiet.
                let grid = follow_tempo_source(tempo_source, deck, recorder);
                // **Everything this frame decided, and only then the `tick` that
                // closes it.** A tick is a terminator rather than a header:
                // `session::split` files each record into the frame of the *next*
                // tick, so a record written after this frame's tick belongs to the
                // next frame. The audio was on the wrong side of that line, which
                // showed a replay frame N what frame N−1 heard.
                measure_audio(audio, deck, recorder, clock.interval(), steps, grid);
                if let Some(recorder) = recorder {
                    recorder.push(karakuri_store::record::Record::Tick { steps });
                }
                frame::Committed { steps, look: *look }
            },
            // The window's frame is its sinks and nothing else — there is no
            // panel over the top of it here. See `frame::compose`.
            |_| {},
        );

        // **Said and not returned on.** A present that failed costs this frame's
        // picture; it does not cost the frame, which has already committed and
        // advanced the deck. Everything below is about the frame rather than
        // about the window — the builds that landed while it ran, the governor
        // that has to hear about them, and the status line that says how many
        // frames a second are arriving — and a run whose window has stopped
        // taking them is exactly when an operator needs to be told the rest.
        if let Err(e) = outcome {
            eprintln!("surface: {e}");
        }

        // A build landing replaces the Set in a slot, and with it the
        // measurement the deck is budgeting against. **A swap is the whole of
        // that list since ADR-0316**: a verdict against stops the slot with the
        // Set the swap installed and moves nothing. Collected here and governed
        // after the drain, because `Deck::events` borrows the deck for as long
        // as it is being read.
        let mut set_changed = false;
        // Collected rather than recorded inside the loop: `Deck::events`
        // borrows the deck for as long as it is read, and writing a record
        // needs the recorder.
        let mut procedures: Vec<(usize, u64)> = Vec::new();
        for slot in 0..self.deck.slot_count() {
            for event in self.deck.events(EngineSlot(slot as u8)) {
                set_changed |= matches!(event, Event::Swapped { .. });
                // **The same words, to whoever is not at the terminal.** A
                // model that wrote a procedure has no other way to learn that
                // its slot was stopped for cost, and "it compiled" is not the
                // same news as "it is on screen and running".
                //
                // Formatted once and only when there is somebody to tell: a run
                // with no `--mcp` used to pay for a `String` it then dropped.
                match &self.mcp {
                    Some(mcp) => {
                        let said = event.to_string();
                        eprintln!("slot {slot}: {said}");
                        mcp.swap(slot, &said);
                    }
                    None => eprintln!("slot {slot}: {event}"),
                }
                // **What a session says it played, at the moment it changed.**
                // The material used to be written once, before the first frame,
                // so a run in which a procedure was rewritten replayed as
                // though it never had — and with a model at the other end of
                // `--mcp` that is the common case rather than a corner.
                if let Event::Swapped { id, .. } = event {
                    procedures.push((slot, id));
                }
            }
        }
        for (slot, landed) in procedures {
            self.took_up(slot, landed);
        }
        if set_changed {
            self.govern("build landed");
        }

        self.frames_since_status += 1;
        if self.status_at.elapsed() >= STATUS_INTERVAL {
            self.print_status();
        }
    }

    /// What the operator needs and nothing that costs a stall to know: which slots
    /// are on air, what they are faded to, where their simulation clocks are, the
    /// output look, and whether frames are still arriving on time.
    ///
    /// Element live counts are deliberately absent — `Set::live_count` blocks until
    /// the queue drains, so a status line carrying it would put a GPU sync on the
    /// render thread twice a second. It is printed once, at exit.
    fn print_status(&mut self) {
        let elapsed = self.status_at.elapsed().as_secs_f32();
        let fps = if elapsed > 0.0 {
            self.frames_since_status as f32 / elapsed
        } else {
            0.0
        };
        self.status_at = Instant::now();
        self.frames_since_status = 0;

        self.status.clear();
        for slot in 0..self.deck.slot_count() {
            let addr = EngineSlot(slot as u8);
            let _ = write!(
                self.status,
                "{}{slot} {} g{:.2} t{:.1}s ",
                if slot == self.focus { ">" } else { " " },
                residency_tag(self.deck.residency(addr), self.deck.is_parked(addr)),
                self.deck.gain(addr),
                self.deck.slot(addr).set().time()
            );
            // **The slot has stopped updating, and only while it has.** The
            // version in it costs more than one frame may, so the engine skips
            // its step and its draw and it holds the frame it last drew
            // (ADR-0316). Printed on the same terms as the transport and the
            // fader below — a run in which no slot is stopped prints the line
            // it always printed — and printed *whole*, not as a four-letter
            // column beside the residency: it is not a residency, a stopped
            // slot is still mixed, and `t` standing still beside `LIVE` is
            // exactly the reading an operator would otherwise take for a bug.
            self.status
                .push_str(stopped_tag(self.deck.overloaded(addr)));
            // **What is moving, and where it is going.** An armed fade is
            // invisible otherwise: with the default quantum it is due up to a
            // bar after the key, and the only thing that said so was one line
            // at press time. A control that changes something invisible is
            // indistinguishable from a control that is broken, which is this
            // file's own argument for printing on every key.
            for t in self.deck.transitions_on(addr) {
                let _ = write!(
                    self.status,
                    "{}>{:.2} ",
                    match t.control() {
                        karakuri_engine::transition::Control::Gain => "g",
                        karakuri_engine::transition::Control::Opacity => "o",
                        karakuri_engine::transition::Control::MaskPosition => "w",
                    },
                    t.to()
                );
            }
            // **And what is waiting to be chosen**, on the same terms and for
            // the same reason: a selection armed for the next bar is invisible
            // between the key and the music, and `r>1` is the only thing on
            // screen that says a renderer is about to change.
            for selection in self.deck.selections_on(addr) {
                let _ = write!(self.status, "r>{} ", selection.renderer());
            }
            // The fader and the mode, **only when they are doing something**,
            // on the same terms as the transport below: a deck nobody has
            // touched prints the line it always printed. Full opacity under
            // `add` is what every slot comes up as, and a column repeating it
            // four times is four columns of nothing to read in the dark.
            if self.deck.opacity(addr) != 1.0 {
                let _ = write!(self.status, "o{:.2} ", self.deck.opacity(addr));
            }
            if self.deck.blend(addr) != Blend::Add {
                let _ = write!(self.status, "{} ", self.deck.blend(addr).name());
            }
            // The transport, and **only when it is doing something**: a deck
            // nobody has synced prints the line it always printed. `free` is
            // the absence of a transport rather than a setting, and a column
            // reading `free` on every slot would be four characters of nothing
            // on a line that has to be read at a glance in the dark.
            let transport = self.deck.transport(addr);
            match transport.sync() {
                Sync::Free => {}
                Sync::Tempo => {
                    let _ = write!(self.status, "T{:.0} ", transport.anchor_bpm());
                }
                Sync::Beat => {
                    let _ = write!(
                        self.status,
                        "B{:.0}{} ",
                        transport.anchor_bpm(),
                        if transport.scrub_beats() == 0.0 {
                            String::new()
                        } else {
                            format!("{:+.2}", transport.scrub_beats())
                        }
                    );
                }
            }
            // The level, which is why the meter exists: two Sets are matched
            // on `m` and `p` warns which one will dominate the mix wherever it
            // lands regardless of its fader. `None` for an off-air slot is the
            // meter saying it has nothing current rather than showing the last
            // thing the slot drew — see `Deck::level`.
            match self.deck.level(addr) {
                Some(level) => {
                    let _ = write!(self.status, "m{:.3} p{:.1}", level.mean, level.peak);
                    // Texels the meter left out because they were not a finite
                    // number, and **only when there are any**. Not a warning:
                    // dividing by a value that reaches zero is an ordinary
                    // thing for a shader to do and what it produces is a
                    // blown-out pixel. It is here because it explains the two
                    // numbers beside it — they are a mean and a peak over the
                    // texels this did not count — and because 3 and 300000 are
                    // a stray sprite and a frame that is gone.
                    if level.bad_texels > 0 {
                        let _ = write!(self.status, " x{}", level.bad_texels);
                    }
                    self.status.push_str("  ");
                }
                None => self.status.push_str("m---- p----  "),
            }
            // What every binding on this slot last wrote. Without it a binding
            // that is doing nothing — an unknown signal, or an invented one at
            // a tenth effect — is indistinguishable from a binding that never
            // attached, which is the whole reason confidence is worth seeing
            // rather than merely being applied. Nothing is printed for a slot
            // with no bindings, so the default status line is unchanged.
            for (key, value) in self.deck.slot(addr).set().bound() {
                let _ = write!(self.status, "{key}={value:.3}  ");
            }
        }
        // What audio is doing, when there is any. A performer cannot tune what
        // they cannot see: the confidence says whether the input is alive, and
        // the phase error says which way the offset wants nudging.
        // **Where the grid is coming from, before what it currently is.** The
        // number an operator needs to trust is the tempo; the thing that tells
        // them whether to trust it is its source, and until now there was only
        // ever one. `link 2p` is a shared grid with two other peers on it;
        // `link 0p` is a source running and alone, which is a different
        // situation from no source at all and has to look different.
        if let Some(source) = &self.tempo_source {
            let _ = write!(
                self.status,
                "| {} {}p{}{} ",
                source.name(),
                source.peers(),
                // **Counted, not swallowed.** Anchors thrown away for
                // unusable numbers mean a source that has gone wrong, and
                // nothing else would ever say so.
                match source.rejected() {
                    0 => String::new(),
                    n => format!(" x{n}"),
                },
                match (source.ended().is_some(), source.playing()) {
                    (true, _) => " GONE",
                    // **Three states and not two.** A source that has greeted
                    // and said nothing else is not a stopped source, and the
                    // two used to print the same thing — which is the pair an
                    // operator would act differently on. Shown and not acted
                    // on, like every other measurement here.
                    (false, None) => " ?",
                    (false, Some(false)) => " stop",
                    (false, Some(true)) => "",
                }
            );
            // **The tempo, when nothing else is going to print it.** The
            // grid's bpm has always lived in the audio group, which is fine
            // while a microphone is the only thing that moves it — and wrong
            // the moment something else does: a run with `--tempo-source` and
            // no `--audio-in` followed a tempo that appeared nowhere on
            // screen. Printed here only when the audio group is absent, so it
            // appears exactly once either way.
            if self.audio.is_none() {
                let _ = write!(
                    self.status,
                    "{:.1}bpm ",
                    self.deck.signals().oscillator().bpm()
                );
            }
        }
        if let Some(audio) = &self.audio {
            let a = audio.status();
            let _ = write!(
                self.status,
                "| e{:.2} on{:.2} c{:.2} | {}{:.1}bpm heard{:.1} c{:.2}{} err{:+.3}b off{:.0}ms ",
                a.energy,
                a.onset,
                a.confidence,
                // The grid's own tempo first, because that is what the picture
                // is actually running at; what the tracker hears second, so a
                // disagreement between the two is visible rather than implied.
                if a.locked { "lock " } else { "free " },
                self.deck.signals().oscillator().bpm(),
                a.estimated_bpm,
                a.estimate_confidence,
                // The tracker's note that this grid might be at half the
                // music's tempo, shown only when the estimate behind it is
                // worth anything. It moves nothing by itself — `.` does, and
                // this is what tells a performer to consider pressing it.
                if a.half_tempo_hint
                    && a.estimate_confidence >= karakuri_audio::lock::GATE_CONFIDENCE
                {
                    " x2?"
                } else {
                    ""
                },
                a.error,
                audio.latency_offset_ms(),
            );
        }
        eprintln!(
            "{}| {} exp {:.2} | {fps:.1} fps",
            self.status,
            op_name(self.look.op),
            self.look.exposure
        );
    }
}

/// What the status line says about a slot the watchdog stopped, and the empty
/// string for every other slot.
///
/// The version in that slot costs more than one frame may, so the engine skips
/// its step and its draw and it holds the frame it last drew (ADR-0316). What
/// an operator would otherwise read is `LIVE` beside a `t` that has stopped
/// moving, which is exactly the reading the maintainer called *"nothing but a
/// bug"*.
///
/// The whole word, and not a four-letter column beside the residency. It is not
/// a residency: a stopped slot that is Live is still mixed, and taking it off
/// air and putting it back leaves it stopped. Printed only while it is true, on
/// the same terms as the transport and the fader — a run in which no slot is
/// stopped prints the line it always printed.
///
/// Pulled out beside [`residency_tag`] for its reason: the distinction it
/// carries is the one thing about it that can be wrong, and checking it should
/// not need a GPU, a window, or a `Deck`.
pub(crate) fn stopped_tag(overloaded: bool) -> &'static str {
    match overloaded {
        true => "overloaded ",
        false => "",
    }
}

/// The status line's four-column form of the same thing [`residency_name`]
/// spells out. Fixed width, so the columns after it do not move.
///
/// Pulled out beside `residency_name` for the reason `slot_in_range` was: the
/// distinction it carries is the one thing about it that can be wrong, and
/// checking it should not need a GPU, a window, or a `Deck`.
pub(crate) fn residency_tag(residency: Residency, parked: bool) -> &'static str {
    match residency {
        Residency::Live => "LIVE",
        Residency::Priming => "prim",
        Residency::Allocated if parked => "park",
        Residency::Allocated => "off ",
    }
}

/// `parked` is [`Deck::is_parked`] for the same slot: Allocated with a standing
/// request to prime. It is not a residency of its own, which is exactly why it
/// has to be passed in — the residency alone cannot tell a deferred request
/// from no request.
///
/// A parked slot and one nobody asked about are the same [`Residency`] and
/// opposite situations, and a surface that names them alike tells the operator
/// their request was discarded when it is being reconsidered every pass.
pub(crate) fn residency_name(residency: Residency, parked: bool) -> &'static str {
    match residency {
        Residency::Live => "live",
        Residency::Priming => "priming (warming, off air)",
        Residency::Allocated if parked => "parked (asked to prime, waiting for room)",
        Residency::Allocated => "allocated (off air)",
    }
}

/// Whether `slot` names one this deck actually has. Pulled out of
/// [`Live::focus_slot`] so the boundary — the neighbour of the off-by-one the
/// digit keys used to have, where a digit equal to or past the slot count must
/// be rejected rather than wrap or panic — is checkable without a window, a
/// GPU, or a `Deck`.
///
/// A digit key names no deck member yet, so this takes the raw `usize` a press
/// or an MCP argument is rather than a [`karakuri_store::record::DeckSlot`] —
/// there is no address here to validate at construction, only a number to check
/// before one can be made. It checks through
/// [`karakuri_store::record::DeckSlot::new`] rather than `slot < slot_count`
/// again, which is the same question `crates/karakuri-environment/src/mix.rs`'s
/// `mix::change` answers for a stream's own `slot` field.
pub(crate) fn slot_in_range(slot: usize, slot_count: usize) -> bool {
    u8::try_from(slot)
        .ok()
        .is_some_and(|slot| DeckSlot::new(slot, slot_count).is_some())
}
