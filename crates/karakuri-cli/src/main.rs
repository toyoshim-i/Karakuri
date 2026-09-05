//! The V1 entry point.
//!
//! One to [`MAX_SLOTS`] pairs of `.kir` files go in, each through parse, type
//! and contract checking, cost estimation, WGSL generation and pipeline
//! creation, and come out as either a window or a PNG. Nothing here is
//! hand-written shader code.
//!
//! Every pair becomes one deck slot with its own `HotSwap`, and the deck
//! composites the Live ones. A window is therefore an instrument rather than a
//! demo: the keyboard puts slots on and off air, moves their gain, and cycles
//! the tone mapper — see [`BINDINGS`], which is both what `--help` prints and
//! what the window prints at startup.
//!
//! This is also the only place a clock is read. The engine advances by `steps`
//! from a `tick` record and never measures anything; deriving that count from
//! elapsed real time is the job of whoever drives the engine live, and on
//! replay it is read back from the stream instead. Keeping the measurement out
//! here is what lets the same engine code be deterministic.

// **The program is not this binary's**, and there is no `mod` line above this
// one any more: all thirteen modules moved to `karakuri-environment` under
// ADR-0214 and ADR-0215, and they are reached here by name so that every call
// site below reads exactly as it did. What is left in this file is the window,
// the arguments, the key handler and `Live` — a surface over them.
use karakuri_environment::{
    audio, compile, history, mcp, midi, mix, places, render, scratch, session, setfile,
    tempo_source, watch,
};
// **Brought into scope rather than reached through their modules**, because
// each was written here and every call site below is the one it already was.
// `Names` crossed with `setfile` in the second slice; the rest crossed with the
// watcher, the MCP server, the mixer and the MIDI map — the material types
// those read a slot off a disk through, the layer spelling a Set file and a
// `--param` share, the save path's own values, the operator's name for a tone
// map, and the sentences P-0090 says belong where every surface can reach them.
// Four more are reached only from the tests below and are brought in there.
use karakuri_environment::compile::{sort_slot, Material, Named, Placed};
use karakuri_environment::mix::{op_name, op_wire_names, parse_op};
use karakuri_environment::setfile::{layer_named, Names, SavedNode, Sources};
use karakuri_environment::{
    accepted_save, no_such_renderer, no_such_slot, nothing_to_save, Asked, SAVE_WAIT,
};

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use karakuri_engine::binding::{Curve, CONTROL_PREFIX, CURVES, DEFAULT_BPM, NOISE_SIGNAL};
use karakuri_engine::deck::MAX_SLOTS;
use karakuri_engine::frame;
use karakuri_engine::swap::Event;
use karakuri_engine::transport::Sync;
use karakuri_engine::{
    Binding, Blend, Deck, Gpu, HotSwap, Look, MaskKind, ParamWrite, Present, Residency, Set,
    Signals, TonemapOp, DEFAULT_BUDGET_MS,
};
use karakuri_operation::Operation;
use karakuri_operation_record::{Current, Written};
use karakuri_signal::NoiseConfig;
use karakuri_store::record::{BindNoise, Layer, Record};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// The fixed simulation step. Not the frame delta.
///
/// The engine's, not a second copy of it: the session oscillator a binding
/// reads advances by this, so a value here that drifted from the engine's would
/// put beats in the wrong place with nothing to show for it.
const DT: f32 = karakuri_engine::set::DT;

/// Past this the simulation falls behind rather than catching up. Unbounded
/// catch-up turns a load spike into a death spiral.
const MAX_STEPS: u8 = 4;

const SEED: u32 = 19_274;

/// How often the status line is printed. Not every frame: at 120 Hz that is a
/// line of stderr per 8 ms, which is unreadable and is I/O on the render
/// thread that nobody asked for.
const STATUS_INTERVAL: Duration = Duration::from_millis(500);

/// One press of a gain key. Linear and additive, because a fader is: the
/// operator wants the same physical move to mean the same amount everywhere,
/// not a proportion of wherever the slot happens to be.
const GAIN_STEP: f32 = 0.1;

/// Where a scheduled fade starts, and what to call it. **A bar is four beats**
/// here, which is an assumption rather than a measurement: nothing in the
/// signal bus knows a time signature, and four is what the `bar` signal already
/// means. A set in three would want this to be a dial, and would say so by
/// having a fade land in the wrong place — which is a better way to find out
/// than a setting nobody knew to change.
const QUANTA: [(f64, &str); 3] = [(4.0, "the next bar"), (1.0, "the next beat"), (0.0, "now")];

/// How long a scheduled fade lasts, in beats. A bar, half a bar, two bars, and
/// a cut — the four an operator reaches for, in the order they are reached for.
const FADE_BEATS: [f64; 4] = [4.0, 2.0, 8.0, 0.0];

/// The shapes a wipe can take, in cycle order, with the angle each runs at and
/// what to call it.
///
/// `None` first, so that a deck nobody has touched wipes with nothing and says
/// so rather than doing something. The rest are the four directions and the
/// iris — the ones a hand reaches for. An arbitrary angle is a dial, and a dial
/// with nowhere to show its value is a control an operator cannot read.
const MASK_SHAPES: [(MaskKind, f32, &str); 6] = [
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
const OPACITY_STEP: f32 = 0.1;

/// One press of an exposure key, as a factor. Multiplicative, because exposure
/// is: a stop is a ratio, and an additive step would be enormous at 0.1 and
/// invisible at 8.0. This is a quarter of a stop, near enough.
const EXPOSURE_STEP: f32 = 1.189_207;

/// **The interactive bounds, and only the interactive ones.** `-` and `=` nudge
/// inside them, because a key that steps has to stop somewhere and a stop it
/// cannot see is worse than one it can. `--exposure` is **not** held to them,
/// which is deliberate and is `exposure_positive_is_accepted_unclamped`'s own
/// sentence: a batch render asks for something extreme on purpose, and a flag
/// is read once by somebody who typed it rather than nudged into a corner. What
/// the flag refuses is what has no meaning at all — see [`clamp_exposure`] for
/// why zero and negative are neither clamped nor accepted anywhere.
///
/// **This comment said the two shared a range and they never have.** It was
/// written beside a constant pair pulled out so the bound would not drift, and
/// the drift was the sentence rather than the numbers.
const EXPOSURE_MIN: f32 = 1.0 / 64.0;
const EXPOSURE_MAX: f32 = 64.0;

/// Exposure is a multiplier before the tone map; zero is degenerate (always
/// black) and negative inverts an otherwise-positive HDR value into one no
/// tone mapper is specified for. Pulled out as a pure function so the bound is
/// one piece of logic instead of two copies that could drift, and so it is
/// testable without a `Live` or a GPU.
fn clamp_exposure(exposure: f32) -> f32 {
    exposure.clamp(EXPOSURE_MIN, EXPOSURE_MAX)
}

/// **A demonstration that drives itself**, as a list of `(seconds, key)`.
///
/// Every entry goes through [`Live::key`], the same function a keyboard reaches,
/// so what a watcher sees is what pressing those keys does and not a second
/// path that resembles it. Nothing here can do anything a person could not.
///
/// It exists because a window is the only honest demonstration of a transport —
/// the scrub is a motion, and a still frame of it is a still frame — and
/// whoever is *describing* the feature is often not the one at the keyboard.
///
/// The order is the argument: engage first and let it sit, so it is clear that
/// engaging changes nothing; then scrub back a long way, hold, and let it run
/// back onto the grid.
const DEMO_SCRIPT: &[(f32, char)] = &[
    // Four seconds of the material as it is, for a before.
    (4.0, 'y'), // beat sync — the picture does not move, deliberately
    // Two bars back, a quarter beat at a time, over about a second and a half.
    (6.0, 'u'),
    (6.1, 'u'),
    (6.2, 'u'),
    (6.3, 'u'),
    (6.4, 'u'),
    (6.5, 'u'),
    (6.6, 'u'),
    (6.7, 'u'),
    (6.8, 'u'),
    (6.9, 'u'),
    (7.0, 'u'),
    (7.1, 'u'),
    (7.2, 'u'),
    (7.3, 'u'),
    (7.4, 'u'),
    (7.5, 'u'),
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
    (17.5, 'f'),
    (22.0, 'g'),
];

/// How long one pass through [`DEMO_SCRIPT`] lasts before it starts over. Past
/// the last entry, so the run ends free-running for a few seconds — the state
/// it began in, which is what makes the next pass legible as a repeat rather
/// than as something new.
const DEMO_LOOP_SECONDS: f32 = 27.0;

/// **The two topologies, over geometry that does not change**, as a list of
/// `(seconds, key)` on the same terms as [`DEMO_SCRIPT`].
///
/// The deck holds one L1 file paired with a sprite renderer in slot 0 and a
/// stroke renderer in slot 1, so **showing each slot without the other is the
/// demonstration**. What a watcher sees is the same cloud, in the same places,
/// at the same instant, drawn two ways.
///
/// **It fades a slot out rather than auditioning the other one, and that is
/// the one thing here that changed.** This script pressed `v` three times —
/// the mix, each slot alone, the mix again — until ADR-0240 retired *Choose
/// what the output shows*. The output is the mix now and always, so the way to
/// see one slot without the other is to take the other out of the mix: `f` on
/// the focused slot's fader, `g` to bring it back, with a digit before each to
/// say which slot. That is `Operation::FadeDeck` where it used to be
/// `SetPreview`, and it isolates a **slot** exactly as the audition did.
///
/// **What it costs is that a fade is scheduled and an audition was not.** A
/// press lands on the current grid rather than in the frame it arrives, so the
/// picture changes a beat or two after the entry that asked for it — which is
/// the same gap [`DEMO_SCRIPT`]'s `f` and `g` already have and say is worth
/// watching for. The seconds below leave room for it.
///
/// The mix comes first and last on purpose. Both slots composited is the state
/// that shows they are the same geometry — the strokes lie along the dots —
/// and each slot alone is what shows how different the two look when the other
/// is not there to anchor it.
///
/// **Nothing here presses a key that only means something to someone who was
/// told what to expect.** Each fade produces a visibly different frame on its
/// own, which is the property [`DEMO_SCRIPT`]'s first entry deliberately does
/// not have and has to say so.
const DEMO_LINES_SCRIPT: &[(f32, char)] = &[
    // Five seconds of the mix: strokes and sprites over each other. Then slot
    // 1's fader out, leaving slot 0 alone — sprites *and* strokes, from one
    // simulation.
    (5.0, '1'),
    (5.0, 'f'),
    // Slot 1 back, slot 0 out: strokes alone, the same elements.
    (10.0, 'g'),
    (10.0, '0'),
    (10.0, 'f'),
    // And back to the mix.
    (15.0, 'g'),
];

/// One pass through [`DEMO_LINES_SCRIPT`], with five seconds of the mix after
/// the last press before it starts over.
const DEMO_LINES_LOOP_SECONDS: f32 = 20.0;

/// Which demonstration `--demo` runs.
///
/// A named script rather than a flag, because there is now more than one thing
/// worth showing and a single `--demo` would have to pick. Each is a
/// demonstration harness and not a feature: both drive [`Live::key`], so
/// neither can do anything a person at the keyboard could not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Demo {
    /// The transport: beat sync engaged, scrubbed two bars back, held, and run
    /// forward past where it was.
    Transport,
    /// `topology lines`: one L1, drawn as sprites and as strokes.
    Lines,
}

impl Demo {
    fn from_name(name: &str) -> Option<Demo> {
        match name {
            "transport" => Some(Demo::Transport),
            "lines" => Some(Demo::Lines),
            _ => None,
        }
    }

    fn script(self) -> &'static [(f32, char)] {
        match self {
            Demo::Transport => DEMO_SCRIPT,
            Demo::Lines => DEMO_LINES_SCRIPT,
        }
    }

    fn loop_seconds(self) -> f32 {
        match self {
            Demo::Transport => DEMO_LOOP_SECONDS,
            Demo::Lines => DEMO_LINES_LOOP_SECONDS,
        }
    }

    /// The deck this demonstration needs, for a run that named no material.
    ///
    /// **A demonstration that requires the operator to assemble the scene is
    /// not one.** `--demo lines` is about two renderers over one geometry, and
    /// a watcher handed a one-slot deck sees the script fade the only thing in
    /// the mix out and back. Overridden the moment any `--set` is given, so
    /// this supplies a scene rather than imposing one.
    fn deck(self) -> Vec<(Named, Vec<Named>)> {
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
type Nodes = Vec<(&'static str, u32, karakuri_store::hash::Hash)>;

/// One press of the scrub keys, in beats. A quarter beat — a sixteenth of a bar
/// in four — which is small enough to place a hit by ear and large enough to
/// hear one press.
const SCRUB_BEATS: f64 = 0.25;

/// A level floor: a negative gain would subtract one slot's light from
/// another's, which is a blend mode rather than a level. Not ceilinged — the
/// pipeline is HDR and values above 1.0 are expected. Pure for the same
/// reason as [`clamp_exposure`].
///
/// `Deck::set_gain` floors too, and the two are not a duplicate. This one
/// decides **what the record says**, so a session replays the value that took
/// effect rather than one the engine quietly corrected; that one guards the
/// engine against every record it did not write, which is the whole of a
/// replay. Deleting either leaves a real hole.
fn clamp_gain(gain: f32) -> f32 {
    gain.max(0.0)
}

/// The tone map cycle `t` steps through. Pure so the cycle — and that it
/// returns to where it started — is checkable without a window.
fn next_tonemap(op: TonemapOp) -> TonemapOp {
    match op {
        TonemapOp::Clamp => TonemapOp::Reinhard,
        TonemapOp::Reinhard => TonemapOp::Aces,
        TonemapOp::Aces => TonemapOp::AgX,
        TonemapOp::AgX => TonemapOp::Clamp,
    }
}

const USAGE: &str = "\
karakuri-cli — a window, or a PNG

usage:
  karakuri-cli [options] [L1.kir L4.kir]

sets — one deck slot each, composited in the order given, at most 4:
  --set L1.kir,L4.kir   name one slot. Repeat for more slots. Every path is
                        sorted by the `kind` it declares: another L1 is a
                        second geometry, an L2 deforms, an L3 is the camera, a
                        `kind Field` is a shape the others can call, an L4
                        draws. Order within a kind is the order given. One
                        camera per slot, and as many fields as you name. Any
                        part may be written `name=file.kir`, which is what an
                        --edge points at; a part written bare is named after
                        its procedure
  L1.kir L4.kir         the same thing, positionally, for one pair. Given
                        alongside --set it becomes the last slot
  (nothing)             examples/drift_shell.kir + examples/soft_points.kir

options:
  --render FILE         render one frame offscreen and stop
  --seq DIR             render every frame to DIR/%05d.png and stop
  --frames N            how many simulation frames (default 240)
  --canvas WxH          what is rendered (default 1920x1080). Fixed for the
                        run, and recorded, so a replay is at the size it was
                        performed at
  --size WxH            the preview window only (default 1280x720). The window
                        fits the canvas into itself and has no say in it.
                        Refused with --render, --seq or --replay
  --capacity N          elements per geometry, overriding every source. Without
                        it each L1 runs at the default its own `capacity`
                        declaration names, and 262144 where a file names none
  --param name=value    a uniform write, applied to every Set, and within one
                        to every node declaring that name
  --param L4:1:name=value
                        the same, addressed at one node — which is how two
                        renderers over one geometry get different values
  --bind FIELDS         attach a signal to a param, applied to every Set.
                        Comma-separated `field=value`, one per field of the
                        `bind` record:
                          layer=L1 key=turbulence signal=energy
                          index=1 (optional; without it, every node of
                            that layer declaring the key)
                          curve=lin|pow2|sqrt|smooth  range=LOW..HIGH
                          noise.kind=white|value|perlin|fbm
                          noise.rate=N  noise.stream=N
                          noise.octaves=N   (needs noise.kind=fbm)
                        layer, key, signal and range are required.
                        signal=bpm is refused: a tempo is not a [0,1] signal
                        and the binding would never move — bind beat or bar
  --edge NODE.SLOT=NODE bind a procedure's declared input to a node of the
                        Set: `--edge morph.far=sphere_shell` for a geometry,
                        `--edge field_lens.shape=melt_blob` for a field. A
                        `.kir` that takes one names the slot and never which
                        node fills it, so this is where that is said — and a
                        slot nothing binds is refused rather than guessed at,
                        as is one bound to the wrong sort of node. Both sides
                        are node names: one you wrote with
                        `--set far=file.kir`, or the procedure's own where you
                        wrote none
  --publish NAME=SPEC   put one control on the console, over a param or a
                        node's param: `level=exposure[0..2]` or
                        `level=L4:0:exposure[0..2]`. Repeat for more. An
                        interface that publishes nothing publishes everything,
                        so the first --publish is what narrows the console
  --merge N             composite slot N's renderers into one image before it
                        reaches the mix, instead of overdrawing them. The slot
                        then takes per-renderer gain, opacity, blend and mask.
                        A Set file records this, so a composited Set saved with
                        --save-set or `k` loads back compositing without the
                        flag — and this flag can only turn it on, so either
                        saying so is enough and the two never disagree
  --bpm N               the tempo the local oscillator free-runs at
                        (default 120). With --audio-in this is where the grid
                        starts and what it falls back to; a tracked tempo
                        corrects it rather than replacing this flag. It is also
                        where the tracker's one-octave window starts, so a value
                        within about 40% of the real tempo settles the octave —
                        and `,`/`.` are the fix when it does not
  --mcp PORT            serve the Model Context Protocol on 127.0.0.1:PORT, so
                        a chat client can read a slot's procedure, rewrite it,
                        and be told what the compiler and the frame budget
                        made of it. **Loopback only, deliberately**: reaching a
                        render machine from elsewhere is `ssh -L`, which is a
                        thing an operator does on purpose. Best with --watch,
                        which is what picks a written procedure up
  --tempo-source CMD    run CMD as a child process and follow the beat it
                        reports. A shared grid carries a beat *number*, so
                        `bar` becomes the room's bar rather than one counted
                        from when this started — which a beat tracker cannot
                        do, because a downbeat is not recoverable from audio.
                        The first anchor aligns the grid and every one after
                        it is trimmed, because the source is another program;
                        while one is attached the beat tracker keeps measuring
                        and stops moving the grid. See docs/plugins.md
  --audio-in NAME       open an audio input: `default`, or any part of a
                        device's name. `energy`, `onset` and band0..7 become
                        measured signals at full confidence, and the beat is
                        tracked and corrected onto the local oscillator
  --latency-offset-ms MS
                        how far the picture leads the sound, beyond the frame
                        queue (default 20, signed). Everything past the two
                        outputs — the PA, the projector, the room — which
                        nothing here can measure. Negative when the sound is
                        the late one. `o`/`p` nudge it live, which is how it is
                        meant to be found: from where the audience stands
  --midi-in NAME        open a MIDI input: any part of a port's name, or an
                        empty string for the first one there is. Every knob
                        goes through the same records a key press writes, so a
                        surface can do nothing a key cannot and a session
                        recorded from one replays with neither attached
  --midi-map FILE       what each knob and pad does, one per line:
                          cc 1 ch 1 -> gain 0        cc 20 -> exposure
                          cc 5      -> opacity 0     note 32 -> residency 0 live
                          note 36 -> residency 0 priming
                          note 40 -> residency 0 allocated
                          note 44 -> blend 0 over    note 48 -> tap
                        A pad names a state, never a step, so a state is a pad:
                        `residency N live|priming|allocated`, `blend N
                        add|over|max`. A file written against the older
                        `on-air N`, `prime N` or bare `blend N` is refused on
                        that line with the line to write instead.
                        `examples/surface.map` is this filled out for four
                        slots, with the reasoning; copy it and edit.
                        `ch` is the number printed on the device, 1-16, and is
                        optional — without it a mapping answers on every
                        channel. A continuous control takes an optional range,
                        `-> gain 0 [0, 2]`, defaulting to [0, 1] for the faders
                        and [0.25, 4] for exposure. Optional even with a port:
                        without a map, every message prints the line that would
                        map it, which is how a surface is discovered
  --tonemap OP          clamp | reinhard | aces | agx (default aces)
  --exposure V          output exposure, before the tone map (default 1.0)
  --store DIR           where artifacts and Set files live (default .karakuri)
  --list-sets           print what the store holds — one line per Set: its id,
                        when it was saved, and how many nodes on each layer —
                        and stop. Nothing is compiled and no window opens
  --save-set ID         put every `.kir` of slot 0 in the store, write the
                        material as a Set file, and stop. It records what the
                        run was *drawing* rather than what the flags said: a
                        capacity and a salt per geometry, the params, the binds,
                        the camera, and an `edge` per bound slot
  --load-set ID         take the material from a Set file rather than from paths
                        and flags — the whole chain, names and edges included.
                        A flag given beside it wins. Anything the file could not
                        carry is printed rather than dropped in silence
  --package ID|FILE     write the Set filed under ID to standard output with
                        every source it names inlined as `src` records, and
                        stop. That file loads on a machine whose store has
                        never held the material, so it is what you send
                        somebody: `--package night01 > night01.kbset`. An
                        artifact this store does not hold refuses the whole
                        bundle naming it, rather than producing a file that
                        looks self-contained and is not.
                        Given a FILE ending in .kset instead — an authoring
                        Set file, which names its .kir files by relative path
                        and lives beside them — every part is read, hashed and
                        put in the store, and the bundle is written from that:
                        `--package night01.kset > night01.kbset`. A part that
                        reaches outside the .kset's own directory is refused
                        by name, absolute paths, `..` and symlinks alike
  --take-in FILE        take a Set file into this store, and stop. A .kbset —
                        what --package writes — has its inlined sources put in
                        the store and its Set file written, and every source
                        must hash to the address its `slot` names or the file
                        is refused whole. A .kset is resolved against its own
                        directory first, behind the same wall --package puts
                        it behind, and then taken in the same way. The id
                        comes from the file either way, and one already taken
                        is refused rather than overwritten
  --record-session ID   write the timeline to sessions/ID.ndjson as it
                        happens: the Set's records, then a `tick` a frame and
                        every edit between them. The material goes at the head
                        either way — from --load-set when given, and saved
                        under ID-material when not — so --replay needs nothing
                        else. Refused with --render, --seq and --replay, which
                        have no performance to record
  --replay ID           render sessions/ID.ndjson instead of running: the
                        material comes from the stream's head and every frame
                        advances by the `tick` that was recorded, so nothing
                        reads a clock. Needs --render or --seq
  --demo NAME           drive the deck from a script instead of the keyboard,
                        so a window shows it without anyone at one. A
                        demonstration harness: it presses keys and can do
                        nothing a person could not.
                          transport   beat sync engaged, scrubbed two bars
                                      back, held, then run forward past where
                                      it was
                          lines       one L1 drawn as sprites and as strokes,
                                      each slot faded out in turn so the other
                                      is seen alone. Brings its own two-slot
                                      deck unless --set says otherwise
  --watch               recompile and swap the slot whose files changed
  --budget-ms MS        frame budget a swapped-in Set is held to
  -h, --help            this

--render and --seq render the mix, which is what the window shows.
";

/// Printed by `--help` and once at startup, because there is no on-screen UI
/// and an instrument whose controls are undiscoverable is not one.
const BINDINGS: &str = "\
keys:
  0-3        focus a slot — the digit is the slot number the status line and
             every swap message use, so there is no off-by-one to remember
  space      focused slot on air / off air. Off air holds its `t`, so it
             resumes where it stopped rather than restarting
  w          ask the focused slot to warm off air, or withdraw the request.
             A request, not a command: the governor grants it only if the
             frame budget has room, and `park` on the status line is a request
             it is still holding — reconsidered every pass, so it takes effect
             by itself when a slot comes off air
  [ ]        focused slot gain down / up — the level the material arrives at,
             colour only, and not clamped at 1.0 because the mix is HDR
  \\          focused slot gain back to 1.0
  ; '        focused slot opacity down / up — the fader across the blend, and
             the only control that silences a slot under every mode. Pull this
             one, not the gain, to get out of material that has gone bad: an
             `over` layer at zero gain is a black card and still covers
  f g        fade the focused slot's fader out / in, over the current length,
             starting on the current grid. Opacity rather than gain: opacity
             silences under every blend mode, where a gain of zero under
             `over` is a black card that still covers
  x          crossfade — the focused slot out and the next one in, together.
             Two scheduled moves sharing a start and a length rather than one
             crossfade object, which is what makes a fade-in, a fade-out and a
             cut the same thing with different numbers. The slot being faded
             in is put on air first
  c          wipe the next slot in over the focused one — a mask at position
             0 on the incoming slot, put on air under `over`, and one
             scheduled move carrying the front to 1. Nothing in the
             transition knows what a mask is and nothing in the mask knows
             what a beat is; a wipe is the two of them. Needs a shape from `z`
  z          cycle the wipe's shape: off, left to right, bottom to top, the
             two diagonals, an iris
  r          cycle which renderer of the focused slot is live, landing on the
             current grid. Needs the slot to composite — --merge, or a Set
             file that records one: overdrawn renderers share one target and
             there is nothing to silence. The ones not selected still draw,
             into targets of their own — the choice is free of a rebuild, not
             free of a frame. One way: there is no position that folds them
             all back together. What you choose is kept: `k` writes it into
             the Set file as the merge's `live`
  n          cycle where a fade starts: the next bar, the next beat, now
  j          cycle how long a fade lasts: 4, 2, 8 beats, or 0 for a cut
  m          cycle the focused slot's blend mode: add, over, max. `over` is
             the only one in which a layer hides the ones under it, and what
             it hides with is the coverage its own sprites drew — thin
             material barely covers, which is not a bug. `screen` and
             `multiply` are absent on purpose: they are defined on [0, 1] and
             nothing has tone mapped this far up the pipeline
  t          cycle the tone map operator: clamp, Reinhard, ACES, AgX
  - =        output exposure down / up
  `          exposure back to 1.0
  y          cycle the focused slot's sync: free, tempo, beat. Modes the
             material cannot take are skipped with the reason — beat sync
             needs closed-form material, and tempo sync is refused on material
             that reads `beats`, which already follows the room. Engaging
             anchors the material at the current tempo, so nothing jumps
  u i        scrub the focused slot back / forward, a quarter beat a press.
             Beat sync only: scrubbing moves a position and the other modes
             are rates
  b          tap the beat — three or more taps set the tempo as well, and a
             tap always sets the phase. Needs --audio-in
  , .        halve / double the grid, and the octave the tracker looks in with
             it. Nothing finds an octave for you: the tracker follows the grid,
             so a set started an octave off stays an octave off until this key.
             `x2?` on the status line is the tracker saying it might be one.
             The phase does not move. Needs --audio-in
  o p        latency offset down / up, 5 ms a press, and it goes negative.
             Raise it if the picture reads late from where the audience is,
             lower it if the sound does. This is the only instrument that can
             read the PA and the projector, so it is the one to trust
  a          size the window so the canvas lands in it one texel to one texel.
             The window is a preview and fits the canvas into itself, so it
             normally shows bars; press this when something downstream is
             capturing the window, because a capture that is neither the
             canvas nor a clean crop of it is worse than useless. The canvas
             itself is --canvas and does not move — resizing the window
             changes what you can see and nothing about what is drawn
  k          keep the focused slot: write what it is playing right now as a
             Set file, named after the moment you pressed this. The capacities,
             the params, the bindings, the camera, the seeds and the sources
             are read off the Set on screen rather than off the flags the run
             started with, so a param you moved and a `.kir` you rewrote are
             both in it. Two are not on the Set to read and come from the run
             instead: the edges, because an edge is consumed where a Set is
             built and no control rewires one, and each node's name, which
             belongs to the way the file was spelled rather than to the
             procedure. The store write happens on a thread of its own and the
             line saying where it went arrives when it lands. `--save-set` is
             the same file written before a run instead of during one
  s          print the status line now
  h ?        print these bindings
  esc        quit
";

/// What a Set file said that no flag can say.
///
/// Not flags: there is no `--seed` and no `--camera`, and inventing two so that
/// a file could be read would be adding surface to carry a value rather than to
/// be used. **Slot 0's, always** — `--load-set` fills that slot and every other
/// comes from `--set`.
#[derive(Clone, Debug, Default)]
struct FromSet {
    /// What each geometry was salted with, by index, `None` where the file
    /// named none.
    ///
    /// **Per geometry rather than one number for the Set**, because that is
    /// what the record says and what the picture depends on: a salt derived
    /// from a source's position in `--set` moves when the list is reordered,
    /// and one read back from a file does not. The first entry doubles as the
    /// Set's own seed — see [`salts_for`].
    salts: Vec<Option<u32>>,
    camera: Option<karakuri_engine::camera::Orbit>,
    /// **Whether the file said its renderers composite**, which `--merge` is
    /// the other way of saying — see [`layering_for`], where the two meet.
    ///
    /// **Kept here rather than pushed into `args.merge`.** Folding it into the
    /// flag would make `--load-set` of a composited Set indistinguishable from
    /// `--merge 0` in every message that prints what the operator asked for,
    /// and the two are different sentences: one is what the file says, the
    /// other is what the hand said.
    layering: karakuri_engine::set::Layering,
    /// **Which renderer the file left folded to**, and `None` where every one
    /// of them is live — see `setfile::Loaded::live`, whose value this is.
    live: Option<u32>,
    /// What each geometry runs at, by index, `None` where the file named none.
    ///
    /// **Kept here rather than folded into `--capacity`.** One flag holds one
    /// number and a Set file holds one per geometry, so folding would put the
    /// first source's count onto every source — which is the bug
    /// [`capacities_for`] exists to have stopped making.
    capacities: Vec<Option<u32>>,
}

#[cfg_attr(test, derive(Debug))]
struct Args {
    /// `--param name=value`, applied after each Set is built, to every Set. A
    /// parameter change is a uniform write, not a structural change, which is
    /// why it needs no fork and no recompilation.
    overrides: Vec<ParamWrite>,
    /// `--bind`, applied to every Set on the same terms as `overrides`.
    ///
    /// **This flag is a stand-in for a Set file and is shaped so it can be
    /// retired for one.** Bindings belong in a `.kbset`, but nothing
    /// loads one into the engine yet — `karakuri-store` decodes records and no
    /// other crate reads a `Record` — and that is its own slice of work. So
    /// the flag names the record's own fields, one comma-separated
    /// `field=value` per JSON field, and the day `--set` takes a Set file the
    /// change here is deleting this and calling `Set::bind` from the decoder
    /// instead. Like `--param`, it applies to every Set, because the record
    /// has no slot field: a Set file is per Set, and one per `--set` is what
    /// replaces it.
    bindings: Vec<Binding>,
    /// `--edge`, applied to every Set on the same terms as `overrides`.
    ///
    /// **A procedure declares a named input slot and the Set binds it to a
    /// node.** `uses far : Geometry` in a `.kir` says what the file needs
    /// without naming which node supplies it — a file that named one would be
    /// coupled to one Set — so this is where the other half is written. Which
    /// geometry a morph blends towards used to be `--set` position 1, written
    /// nowhere at all, and reordering the command line changed the picture in
    /// silence.
    ///
    /// **The record is the home and the flag writes into it**, on the terms
    /// `--bind` follows: an `edge` record in a Set file is what a saved use
    /// carries, and this is the authoring surface that exists today. Like
    /// `--param` it applies to every Set of the deck, and an edge whose node
    /// names nothing in a given Set is a statement about a different one — see
    /// [`karakuri_engine::set::Wiring::edges`].
    edges: Vec<karakuri_engine::set::Edge>,
    /// The session tempo. There is **no tempo record** in the v0.2 vocabulary,
    /// so unlike `--bind` this flag has nothing to map onto yet; it is here
    /// because a binding to `beat` is meaningless at a tempo nobody can set.
    bpm: f32,
    /// One deck slot per entry, in composite order: the L1 that simulates, and
    /// the renderers drawn over it in list order — see `Set::build_many`. One
    /// renderer is the ordinary case; several is one simulation drawn several
    /// ways, which costs a draw pass apiece and no extra memory.
    sets: Vec<(Named, Vec<Named>)>,
    /// What `--capacity` was given, or [`karakuri_ir::DEFAULT_CAPACITY`] when
    /// it was not — read only through [`capacity_for`], which prefers the
    /// procedure's own declaration to this unless `capacity_given`.
    capacity: u32,
    /// Whether `--capacity` was *typed*. Without it a procedure's own declared
    /// default is used — see [`capacity_for`].
    capacity_given: bool,
    render_to: Option<PathBuf>,
    seq_to: Option<PathBuf>,
    frames: u32,
    /// **The preview window's size, and nothing else.** A window is a preview
    /// of what leaves by some other route, so it has no say in what is drawn —
    /// see `canvas`. Meaningless without a window, and refused rather than
    /// ignored when there is none.
    size: (u32, u32),
    /// **What the run renders at.** The canvas every `VideoSource` draws into,
    /// what every deck slot is sized to match, and what an offscreen render
    /// writes. Fixed for the run: changing it reallocates every slot's target,
    /// and the frame path allocates nothing.
    canvas: (u32, u32),
    /// Whether `--canvas` was *typed*. Carried rather than resolved at parse
    /// time because what it is refused against — whether the session being
    /// replayed carries a canvas of its own — is not known until the stream is
    /// read. See `replay_session`.
    canvas_given: bool,
    /// Watch every pair and hot-swap the slot whose files changed. Off by
    /// default: a run that is not being edited should not carry a worker
    /// thread per slot and a watchdog it will never use.
    watch: bool,
    /// Which slots composite their renderers rather than overdrawing them —
    /// `--merge 0`, repeatable. See `karakuri_engine::set::Layering`.
    ///
    /// **Not the whole answer any more**: a Set file records its layering, so a
    /// slot filled by `--load-set` may composite without appearing here. Ask
    /// [`layering_for`], which is where the flag and the file meet.
    merge: Vec<usize>,
    /// The interface every slot's Set publishes — `--publish
    /// name=L4:0:exposure[0.2..0.8]`, repeatable. Empty means every Set
    /// publishes everything it declares, which is what happened before an
    /// interface existed.
    published: Vec<karakuri_engine::set::Published>,
    /// `--mcp`. A port to serve the Model Context Protocol on, loopback only.
    /// `None` is the ordinary case and nothing in the frame path changes: this
    /// is a third control surface beside the keyboard and MIDI, and like them
    /// it can do nothing they cannot.
    mcp: Option<u16>,
    /// `--tempo-source`. A command to run as a child process that says where
    /// the beat is — see [`crate::tempo_source`]. `None` is the ordinary case
    /// and changes nothing: the grid comes from `--bpm`, the beat tracker and
    /// the tap keys exactly as it always has.
    tempo_source: Option<String>,
    /// `--audio-in`. `None` is no device at all, which is not the same as a
    /// device that is silent: with no device the bus answers `energy` and the
    /// bands exactly as it did before audio existed, and the oscillator
    /// free-runs.
    audio_in: Option<String>,
    /// `--midi-in`: a substring of the input port's name, or empty for the
    /// first one there is.
    midi_in: Option<String>,
    /// `--midi-map`: the operator's table. Optional even with a port, because
    /// a surface with no map still prints what it sends, which is the state an
    /// operator is in before they have written one.
    midi_map: Option<PathBuf>,
    /// The unmeasurable half of the output lag, in milliseconds. See
    /// `audio::DEFAULT_LATENCY_OFFSET_MS`.
    latency_offset_ms: f32,
    /// `--store DIR`: where content-addressed artifacts and Set files live.
    store: PathBuf,
    /// `--save-set ID`: write the material as a Set file and stop.
    save_set: Option<String>,
    /// `--load-set ID`: take the material from a Set file instead of from two
    /// `.kir` paths and the flags.
    load_set: Option<String>,
    /// `--record-session ID`: write the timeline as it happens.
    record_session: Option<String>,
    /// `--replay ID`: render a recorded session instead of running one.
    replay: Option<String>,
    /// What a Set file said that no flag says, when one was loaded — see
    /// [`FromSet`].
    from_set: Option<FromSet>,
    /// Drive the deck from a named script instead of waiting for a keyboard.
    /// A demonstration harness, not a feature: it presses keys.
    demo: Option<Demo>,
    /// The frame budget the watchdog holds a swapped-in Set to, in
    /// milliseconds. Exposed mostly so that rollback can be provoked on
    /// demand — `--budget-ms 0` rejects everything — rather than only by
    /// writing a procedure slow enough to trip it.
    budget_ms: f32,
    look: Look,
}

fn fail(message: &str) -> ! {
    eprintln!("karakuri-cli: {message}\ntry `--help`");
    std::process::exit(2);
}

/// What [`parse_args_from`] found, short of a full [`Args`]: `--help` is a
/// request to print and exit successfully, not an error.
enum ParseOutcome {
    Run(Box<Args>),
    Help,
    /// `--list-sets`: print what the store at this path holds, and stop.
    ///
    /// **A third outcome rather than a field on [`Args`]**, and for `--help`'s
    /// reason: this is not a run. Nothing downstream of here — the compile, the
    /// scratch, the deck, the window — has anything to do with a listing, and a
    /// flag carried into [`Args`] would have to be checked above every one of
    /// them and would be wrong the day somebody added one more. The variant
    /// makes reaching a GPU with this flag not a thing that can be forgotten:
    /// there is no `Args` to run.
    ///
    /// It carries the store path because that is the whole of what a listing
    /// needs, and `--store` may be given on either side of this flag.
    ListSets(PathBuf),
    /// `--package ID` — or `--package FILE.kset`: write the Set filed under
    /// `ID`, or the one the authoring file at `FILE` names, to standard output
    /// with every source it names inlined, and stop.
    ///
    /// **One flag and two moments rather than two flags**, which is ADR-0229
    /// part 4: packaging is *"one operation, two moments"*, and
    /// `docs/manual/operations.html`'s *Send a Set to somebody, and take one
    /// in* is the row both are a moment of. The field is still `id` because the
    /// parse cannot tell which it is without touching a disk and does not try;
    /// [`packaged_set`] decides on the extension, which is what ADR-0231 made
    /// the extension for.
    ///
    /// **And the flag is named for what it does at both moments.** It writes a
    /// store as often as it reads one — a `.kset` has every part hashed and put
    /// — so `--bundle`, which reads as an export and nothing else, named half
    /// of it. `--package` and [`ParseOutcome::TakeIn`] are the two directions of
    /// one row (`docs/contributing.md` §4: a name means one thing, and one
    /// thing has one name).
    ///
    /// [`ParseOutcome::ListSets`]'s variant for [`ParseOutcome::ListSets`]'s
    /// reason, and the reason is the whole of why it is here: packaging reads a
    /// file and hashes some bytes, and there is no Set to build, no adapter to
    /// request and no window to open. An `Args` field would have to be checked
    /// above every early return in `main` and would be wrong the day somebody
    /// added one more; a variant with no `Args` in it makes reaching a device
    /// not a thing that can be forgotten.
    Package {
        store: PathBuf,
        id: String,
    },
    /// `--take-in FILE`: take a Set file into the store, and stop — a `.kbset`
    /// as it stands, a `.kset` resolved against its own directory first.
    ///
    /// Here for the same reason, and it compiles: each source goes through the
    /// checker so that a metadata card can be written for it. That is the check
    /// pass, which takes no device — `Set::validate` runs without one and a
    /// single procedure certainly does.
    ///
    /// **Which of the two forms it is, the extension says**, exactly as
    /// [`ParseOutcome::Package`]'s does: the parse carries a path and touches no
    /// disk to classify it, and [`taken_in_file`] decides.
    TakeIn {
        store: PathBuf,
        file: PathBuf,
    },
}

/// The real entry point: reads the process's own arguments, then hands them
/// to [`parse_args_from`] and turns its `Result` into the exit this binary
/// actually makes. Kept this thin so the parsing logic itself takes any
/// iterator of strings and returns rather than exits — which is what makes it
/// possible to drive adversarial input through it in a test.
fn parse_args() -> Args {
    match parse_args_from(std::env::args().skip(1)) {
        Ok(ParseOutcome::Run(args)) => *args,
        Ok(ParseOutcome::Help) => {
            print!("{USAGE}\n{BINDINGS}");
            std::process::exit(0);
        }
        // **Nothing is built to answer this.** The store is opened, the `sets/`
        // directory is read, each file in it is read, and the process ends —
        // see [`listed_sets`]. On stdout, because it is the answer to the
        // question that was asked rather than a note about a run.
        Ok(ParseOutcome::ListSets(root)) => match listed_sets_at(&root) {
            Ok(said) => {
                print!("{said}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("karakuri-cli: {e}");
                std::process::exit(1);
            }
        },
        // **To standard output, so a shell can redirect it.** What packaging
        // produces is a thing you *send somebody* — a single self-contained
        // file that works in their store — rather than something the library
        // keeps, so it goes where `> patch.ndjson` puts it and needs no naming
        // rule of its own in `sets/`. The store already holds this Set under an
        // id; a second copy of it there under some derived name would be a
        // second answer to which file is the Set.
        Ok(ParseOutcome::Package { store, id }) => match packaged_set(&store, &id) {
            Ok(said) => {
                print!("{said}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("karakuri-cli: {e}");
                std::process::exit(1);
            }
        },
        // On stdout for the reason a listing is: it is the answer to the
        // question that was asked. Nothing is compiled to *run* — the checker
        // is reached only to write each artifact's metadata card.
        Ok(ParseOutcome::TakeIn { store, file }) => match taken_in_file(&store, &file) {
            Ok(said) => {
                print!("{said}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("karakuri-cli: {e}");
                std::process::exit(1);
            }
        },
        Err(message) => fail(&message),
    }
}

/// The whole of argument parsing, as a pure function: no I/O, no exit, just
/// strings in and either an [`Args`] or an error message out. Every message a
/// person can act on is produced here rather than by falling back to a
/// default that hides a typo — `--set` and `--tonemap` already worked this
/// way; `--exposure` now validates on the same terms rather than silently
/// keeping 1.0 for a negative, zero, or unparsable value.
/// The value belonging to `flag`, refused rather than defaulted.
///
/// Two silences this removes. A flag at the end of the line with nothing after
/// it used to take `None` and fall back — `--render` with the path forgotten
/// opened a *window*, so a batch script with a typo hung waiting for one
/// instead of failing. And a flag whose value was missing used to swallow the
/// next flag: `--set --render out.png` read `--render` as a pair of paths and
/// then blamed `--render` for not being one.
///
/// A leading `-` is treated as another option unless the whole token parses as
/// a number, so a negative value is still a value.
fn value_for(flag: &str, it: &mut impl Iterator<Item = String>) -> Result<String, String> {
    match it.next() {
        Some(v) if !v.starts_with('-') || v.parse::<f64>().is_ok() => Ok(v),
        Some(v) => Err(format!(
            "`{flag}` was given no value — `{v}` is an option, not one"
        )),
        None => Err(format!("`{flag}` needs a value")),
    }
}

/// A number for `flag`, refused rather than defaulted.
///
/// `--frames`, `--capacity` and `--budget-ms` used to keep their default on
/// anything unparsable, so `--frames 24O` (a letter O) rendered 240 frames and
/// said nothing. A run that quietly used the default is indistinguishable from
/// one that honoured what was typed, which is the same failure this codebase
/// keeps finding in other clothes.
fn number_for<T: std::str::FromStr>(
    flag: &str,
    what: &str,
    it: &mut impl Iterator<Item = String>,
) -> Result<T, String> {
    let value = value_for(flag, it)?;
    value
        .parse()
        .map_err(|_| format!("`{flag} {value}` — expected {what}"))
}

/// A `WIDTHxHEIGHT` pair for `flag`.
///
/// **Zero is refused rather than clamped**, which is the difference between a
/// typo and a picture: everything downstream takes `max(1)` to keep a texture
/// descriptor legal, so `--canvas 1920x0` would have rendered a one-texel-tall
/// frame and reported the size it was asked for.
fn extent(flag: &str, value: String) -> Result<(u32, u32), String> {
    let bad = || format!("`{flag} {value}` — expected `WIDTHxHEIGHT`");
    let (w, h) = value.split_once('x').ok_or_else(bad)?;
    match (w.parse::<u32>(), h.parse::<u32>()) {
        (Ok(w), Ok(h)) if w > 0 && h > 0 => Ok((w, h)),
        (Ok(_), Ok(_)) => Err(format!("`{flag} {value}` — neither side may be zero")),
        _ => Err(bad()),
    }
}

/// One `--bind` value, as the `bind` record's own fields.
///
/// The grammar is `field=value`, comma separated, and every field name is the
/// record's. The single deviation is `range`, which the record writes as a
/// two-element array and this writes as `LOW..HIGH` — a comma inside a value
/// would be indistinguishable from the separator between fields, and quoting
/// rules to fix that would be a second grammar rather than a smaller one.
///
/// Unknown fields are refused rather than ignored. The record format ignores
/// an unknown `t` for forward compatibility between engine versions; a typo on
/// a command line has no such excuse, and `curv=pow2` silently taking the
/// default curve is the exact silence every other flag here was fixed for.
/// `--edge <node>.<slot>=<node>` — bind one procedure's declared input slot
/// to a node of the Set, whatever type the slot was declared with.
///
/// **`.` between the node and the slot, `=` before the node it is bound to.**
/// The `=` is `--set`'s already and means "the thing on the left is a name for
/// the thing on the right"; the `.` is the same dot the procedure reads the slot
/// through, so `--edge morph.far=sphere_shell` and `far.position` in the
/// `deform` are visibly one spelling. A Field slot is read as a call rather than
/// through a dot — `--edge field_lens.shape=melt_blob`, then `shape(p)` — and
/// still writes the same record, because what an edge says is the same fact
/// whatever fills the slot. `:` was not available — `--param` and
/// `--publish` use it for a layer and an index, and it is a path character on
/// Windows.
///
/// **`--bind` was taken**, by signals, which is the other reason the word here
/// is `edge`: it is what the record has always been going to be called, since
/// what it writes down is one edge of the graph a Set describes.
///
/// Every part is refused empty rather than accepted and resolved to nothing: an
/// edge with no slot in it is a sentence about a node, and there is no such
/// sentence.
fn parse_edge(value: &str) -> Result<karakuri_engine::set::Edge, String> {
    let bad = |what: &str| format!("`--edge {value}` — {what}");
    let Some((from, to)) = value.split_once('=') else {
        return Err(bad(
            "expected `<node>.<slot>=<node>`, e.g. `morph.far=sphere_shell` or \
             `field_lens.shape=melt_blob`",
        ));
    };
    // **The last dot, not the first.** A node name may hold one — nothing
    // refuses `--set my.morph=morph.kir` — and the slot is a `.kir` identifier,
    // which cannot.
    let Some((node, slot)) = from.rsplit_once('.') else {
        return Err(bad(
            "expected a `.` between the node and the slot it declares, e.g. \
             `morph.far=sphere_shell`",
        ));
    };
    if node.is_empty() || slot.is_empty() || to.is_empty() {
        return Err(bad("every part names something: `<node>.<slot>=<node>`"));
    }
    Ok(karakuri_engine::set::Edge {
        node: node.to_string(),
        slot: slot.to_string(),
        to: to.to_string(),
    })
}

fn parse_bind(value: &str) -> Result<Binding, String> {
    let bad = |what: &str| format!("`--bind {value}` — {what}");

    let mut layer = None;
    // **Absent is a wildcard, not zero.** A binding with no `index` is the
    // layer's — every node declaring the key — which is what `--bind` has
    // always meant and what one published control would drive. See
    // `Binding::index`.
    let mut index: Option<u32> = None;
    let mut key = None;
    let mut signal = None;
    let mut curve = Curve::Lin;
    let mut range = None;
    // Built whether or not it is used: a `noise.*` field on a binding whose
    // signal is not `noise` is a mistake worth reporting, and that check needs
    // to know one was given.
    let mut noise = NoiseConfig::default();
    let mut noise_given = false;
    // Kept as what was written rather than folded into `noise.kind` as it is
    // read. `NoiseKind` carries the octave count inside the `fbm` variant, so
    // assigning either field as it arrives lets the later one decide the
    // other: `noise.octaves` would turn a `white` that was asked for into an
    // `fbm` that was not. Both are resolved once, after the loop, where the
    // pair can be checked against each other.
    let mut noise_kind: Option<&str> = None;
    let mut noise_octaves: Option<u32> = None;

    for field in value.split(',') {
        let (name, v) = field
            .split_once('=')
            .ok_or_else(|| bad(&format!("`{field}` is not `field=value`")))?;
        let number = |what: &str| -> Result<f32, String> {
            v.parse::<f32>()
                .map_err(|_| bad(&format!("`{name}` expects {what}, got `{v}`")))
        };
        match name.trim() {
            "layer" => {
                layer = Some(layer_named(v).ok_or_else(|| {
                    bad(&format!("`layer={v}` — expected L1, L2, L3, L4 or Field"))
                })?)
            }
            "index" => {
                index = Some(v.parse::<u32>().map_err(|_| {
                    bad(&format!(
                        "`index={v}` — expected a node number, 0 for the first"
                    ))
                })?)
            }
            "key" => key = Some(v.to_string()),
            "signal" => signal = Some(v.to_string()),
            "curve" => {
                curve = Curve::parse(v).ok_or_else(|| {
                    let names: Vec<&str> = CURVES.iter().map(|c| c.name()).collect();
                    bad(&format!("`curve={v}` — expected {}", names.join(", ")))
                })?
            }
            "range" => {
                let (low, high) = v
                    .split_once("..")
                    .ok_or_else(|| bad(&format!("`range={v}` — expected `LOW..HIGH`")))?;
                match (low.parse::<f32>(), high.parse::<f32>()) {
                    (Ok(low), Ok(high)) => range = Some([low, high]),
                    _ => return Err(bad(&format!("`range={v}` — expected `LOW..HIGH`"))),
                }
            }
            "noise.kind" => {
                noise_given = true;
                if !NOISE_KINDS.contains(&v) {
                    return Err(bad(&format!(
                        "`noise.kind={v}` — expected white, value, perlin or fbm"
                    )));
                }
                noise_kind = Some(v);
            }
            "noise.rate" => {
                noise_given = true;
                noise.rate = number("a number of cycles per beat")?;
            }
            "noise.stream" => {
                noise_given = true;
                noise.stream = v
                    .parse::<u64>()
                    .map_err(|_| bad(&format!("`noise.stream={v}` — expected a whole number")))?;
            }
            "noise.octaves" => {
                noise_given = true;
                noise_octaves =
                    Some(v.parse::<u32>().map_err(|_| {
                        bad(&format!("`noise.octaves={v}` — expected a whole number"))
                    })?);
            }
            other => {
                return Err(bad(&format!(
                    "unknown field `{other}` — expected layer, key, signal, curve, range, \
                     or noise.kind / noise.rate / noise.stream / noise.octaves"
                )))
            }
        }
    }

    let layer = layer.ok_or_else(|| bad("no `layer=`"))?;
    let key = key.ok_or_else(|| bad("no `key=`"))?;
    let signal = signal.ok_or_else(|| bad("no `signal=`"))?;
    // Required, unlike `curve`: there is no defensible default range. A param
    // declares its own in the `.kir`, and silently binding across all of it
    // would be an aesthetic decision made by the argument parser.
    let range = range.ok_or_else(|| bad("no `range=LOW..HIGH`"))?;

    // **Built as the record and decoded back**, so the flag is what its
    // documentation always claimed: a way to write a `bind` record. Every
    // semantic rule — the `bpm` refusal, `octaves` needing `fbm`, a generator
    // needing `signal=noise` — lives in `setfile::binding_from_record` and
    // cannot differ between a command line and a Set file. That debt against
    // the decoder is paid by having one rule rather than two copies of it —
    // see `docs/adr/0066-a-flag-becomes-a-record-writer.md`.
    //
    // The one check that stays here is the one the record cannot express.
    // `BindNoise::octaves` has a serde default, deliberately — "a generator
    // omitted field by field is under-specified, not refused" — so a record
    // cannot say whether `octaves` was *named*. The flag knows, and an
    // `octaves` named beside a kind that has no octaves is an operator
    // expecting a generator they did not ask for.
    if noise_octaves.is_some() && noise_kind != Some("fbm") {
        return Err(bad(&format!(
            "`noise.octaves` needs `noise.kind=fbm`, and this asks for `{}`",
            noise_kind.unwrap_or("perlin")
        )));
    }

    let record = Record::Bind {
        layer: record_layer(layer),
        index,
        key,
        signal,
        curve: curve.name().to_string(),
        range,
        noise: noise_given.then(|| BindNoise {
            kind: noise_kind.unwrap_or("perlin").to_string(),
            rate: noise.rate,
            stream: noise.stream,
            octaves: noise_octaves.unwrap_or(setfile::DEFAULT_OCTAVES),
        }),
    };
    setfile::binding_from_record(&record).map_err(|e| bad(&e))
}

/// The `noise.kind` names, in the order the spec lists them. One list, so the
/// check and the message cannot drift apart.
const NOISE_KINDS: [&str; 4] = ["white", "value", "perlin", "fbm"];

/// `--param [L4:N:]name=value`.
///
/// **The address is optional and is `layer:index:` when it is there.** A bare
/// name is a wildcard — every node declaring it, "the Set's `exposure`", one
/// knob moving both renderers — which is what this flag has always meant and is
/// the useful default. The prefix is what sets two renderers apart, and it is
/// present or absent as a unit for the reason `ParamWrite::at` gives: a layer
/// alone stopped naming a node when a Set gained a list of them, so a
/// half-address would be a wish rather than an address.
///
/// `:` cannot occur in a param name — identifiers are alphanumerics and
/// underscores — so splitting on it is unambiguous and needs no quoting. A
/// malformed override is refused rather than dropped, on the same terms every
/// other flag here is: silence looks exactly like a parameter that was applied
/// and had no visible effect. A name that no procedure declares is still only a
/// warning at build time — that one is a question about the `.kir`.
/// A layer name as an operator writes it. **One reader, so `--param` and
/// `--bind` cannot disagree about which layers a Set has** — they did, and the
/// disagreement was silent: `--param L2:…` was refused as a malformed address
/// while `--bind layer=L2` was refused with a sentence saying L2 did not exist
/// yet, both long after it did.
/// A layer as the record vocabulary spells it.
///
/// **Exhaustive on purpose.** A sixth `Kind` stops compiling here rather than
/// falling to a default, which is what the two places that used to map this by
/// hand could not promise.
fn record_layer(kind: karakuri_ir::Kind) -> Layer {
    match kind {
        karakuri_ir::Kind::L1 => Layer::L1,
        karakuri_ir::Kind::L2 => Layer::L2,
        karakuri_ir::Kind::L3 => Layer::L3,
        karakuri_ir::Kind::L4 => Layer::L4,
        karakuri_ir::Kind::Field => Layer::Field,
    }
}

/// `--publish name=L4:0:exposure[0.2..0.8]`, or `--publish level=exposure[0..2]`
/// for every node that declares it.
///
/// **The address is the `--param` one and the range is the `--bind` one**, which
/// is why neither half needed a grammar of its own: what a published control is,
/// is a name in front of an address and a range behind it — and the address is
/// `layer:index:` present or absent as a unit, meaning the same thing it means
/// there. The range is mandatory: publishing without one would mean "over the
/// declared range", and spelling that as an absence would make the common
/// narrowing case look like the exception.
fn parse_publish(value: &str) -> Result<karakuri_engine::set::Published, String> {
    let bad = || {
        format!(
            "`--publish {value}` — expected `name=key[LOW..HIGH]` or `name=L4:0:key[LOW..HIGH]`"
        )
    };
    let (name, rest) = value.split_once('=').ok_or_else(bad)?;
    if name.is_empty() {
        return Err(bad());
    }
    let (at, rest) = match rest.split_once(':') {
        Some((layer, tail)) => {
            let layer = layer_named(layer).ok_or_else(bad)?;
            let (index, tail) = tail.split_once(':').ok_or_else(bad)?;
            let index: u32 = index.parse().map_err(|_| bad())?;
            (Some((layer, index)), tail)
        }
        None => (None, rest),
    };
    let (key, range) = rest.split_once('[').ok_or_else(bad)?;
    let range = range.strip_suffix(']').ok_or_else(bad)?;
    let (low, high) = range.split_once("..").ok_or_else(bad)?;
    let low: f32 = low.parse().map_err(|_| bad())?;
    let high: f32 = high.parse().map_err(|_| bad())?;
    if key.is_empty() {
        return Err(bad());
    }
    Ok(karakuri_engine::set::Published {
        name: name.to_string(),
        at,
        key: key.to_string(),
        range: [low, high],
    })
}

fn parse_param(value: &str) -> Result<ParamWrite, String> {
    let bad = || format!("`--param {value}` — expected `name=number` or `L4:1:name=number`");
    let (addressed, rest) = match value.split_once(':') {
        Some((layer, rest)) => {
            let kind = layer_named(layer).ok_or_else(bad)?;
            // **The index is required, even where a layer can hold only one
            // node.** `L3:0:radius` for a camera a Set has exactly one of is
            // two characters of ceremony, and the rule it keeps is worth more:
            // the address is `layer:index:` present or absent as a *unit*, so
            // there is exactly one wildcard spelling — a bare name. Making the
            // index optional would give `L4:exposure` a third meaning, sitting
            // between "every renderer" and "renderer 0".
            let (index, rest) = rest.split_once(':').ok_or_else(bad)?;
            let index: u32 = index.parse().map_err(|_| bad())?;
            (Some((kind, index)), rest)
        }
        None => (None, value),
    };
    let (key, number) = rest.split_once('=').ok_or_else(bad)?;
    let number: f32 = number.parse().map_err(|_| bad())?;
    if key.is_empty() {
        return Err(bad());
    }
    Ok(ParamWrite {
        at: addressed,
        key: key.to_string(),
        value: number,
    })
}

fn parse_args_from(args: impl Iterator<Item = String>) -> Result<ParseOutcome, String> {
    let mut args_out = Args {
        overrides: Vec::new(),
        bindings: Vec::new(),
        edges: Vec::new(),
        bpm: DEFAULT_BPM,
        sets: Vec::new(),
        capacity: karakuri_ir::DEFAULT_CAPACITY,
        capacity_given: false,
        render_to: None,
        seq_to: None,
        frames: 240,
        size: (1280, 720),
        canvas: (1920, 1080),
        canvas_given: false,
        watch: false,
        merge: Vec::new(),
        published: Vec::new(),
        mcp: None,
        tempo_source: None,
        audio_in: None,
        midi_in: None,
        midi_map: None,
        latency_offset_ms: audio::DEFAULT_LATENCY_OFFSET_MS,
        budget_ms: DEFAULT_BUDGET_MS,
        demo: None,
        store: PathBuf::from(places::STORE),
        save_set: None,
        load_set: None,
        record_session: None,
        replay: None,
        from_set: None,
        look: Look {
            op: TonemapOp::Aces,
            exposure: 1.0,
            white_point: 1.0,
        },
    };
    // Whether this was *typed*, not what it holds: it has a default, so "is it
    // still 1280x720" cannot tell a flag that was given from one that was not,
    // and the refusal below is about the giving. `--canvas` needs the same fact
    // for longer and carries it on `Args` instead.
    let mut size_given = false;
    let mut list_sets = false;
    let mut package: Option<String> = None;
    let mut take_in: Option<PathBuf> = None;
    let mut positional = Vec::new();
    let mut it = args;
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(ParseOutcome::Help),
            "--set" => {
                let value = value_for("--set", &mut it)?;
                // **The first is the L1 and the rest are renderers**, drawn in
                // the order given. A third part used to be refused: it could
                // only be a typo when a Set was a pair, and taking `b,c` as one
                // literal filename would have blamed a missing file for a stray
                // comma. It is now what asking for two renderers looks like, and
                // there is no new syntax for it — one comma-separated list, read
                // as one L1 and however many L4s.
                let parts: Vec<&str> = value.split(',').collect();
                match parts.split_first() {
                    Some((l1, l4s))
                        if !l1.is_empty()
                            && !l4s.is_empty()
                            && l4s.iter().all(|p| !p.is_empty()) =>
                    {
                        // **Each part may carry a name** — `near=lattice.kir`.
                        // A name is what everything downstream addresses the
                        // node by; see `Named`.
                        let head = Named::parse(l1)?;
                        let rest: Vec<Named> = l4s
                            .iter()
                            .map(|p| Named::parse(p))
                            .collect::<Result<_, _>>()?;
                        args_out.sets.push((head, rest))
                    }
                    _ => {
                        return Err(format!(
                            "`--set {value}` — expected `L1.kir,L4.kir`, or \
                             `L1.kir,L4.kir,L4.kir` for several renderers over one geometry. \
                             Any part may be written `name=file.kir`"
                        ))
                    }
                }
            }
            "--render" => args_out.render_to = Some(PathBuf::from(value_for("--render", &mut it)?)),
            "--seq" => args_out.seq_to = Some(PathBuf::from(value_for("--seq", &mut it)?)),
            "--param" => {
                // A malformed override used to be dropped in silence, which
                // looks exactly like a parameter that was applied and had no
                // visible effect. A name that no procedure declares is still
                // only a warning at build time — that one is a question about
                // the `.kir`, not about the command line.
                let value = value_for("--param", &mut it)?;
                args_out.overrides.push(parse_param(&value)?);
            }
            "--bind" => {
                let value = value_for("--bind", &mut it)?;
                args_out.bindings.push(parse_bind(&value)?);
            }
            "--edge" => {
                let value = value_for("--edge", &mut it)?;
                args_out.edges.push(parse_edge(&value)?);
            }
            "--bpm" => {
                let value = value_for("--bpm", &mut it)?;
                // Refused rather than clamped here, on the same terms as
                // `--exposure`: the oscillator clamps a bad tempo so that a
                // zero cannot freeze every noise signal at once, but a tempo
                // typed at the command line and quietly changed is a run that
                // did not do what it was told.
                match value.parse::<f32>() {
                    Ok(v) if v.is_finite() && v > 0.0 => args_out.bpm = v,
                    _ => return Err(format!("`--bpm {value}` — expected a positive number")),
                }
            }
            "--tonemap" => {
                let value = value_for("--tonemap", &mut it)?;
                args_out.look.op = parse_op(&value)
                    .ok_or_else(|| format!("`--tonemap {value}` — expected {}", op_wire_names()))?;
            }
            "--exposure" => {
                let value = value_for("--exposure", &mut it)?;
                // Validated rather than defaulted on failure — a negative,
                // zero, NaN, infinite, or unparsable exposure used to be
                // silently swapped for 1.0, which reads as "the flag was
                // ignored" rather than "the value was wrong". `--tonemap` and
                // `--set` already fail loudly on a bad value; this does the
                // same. **What it does not do is clamp**: `100` is accepted and
                // lands at 100, well outside what `-`/`=` reach, because a
                // batch render is allowed to ask for something extreme. See
                // `exposure_positive_is_accepted_unclamped` and
                // [`EXPOSURE_MIN`], whose comment claimed the opposite.
                match value.parse::<f32>() {
                    Ok(v) if v.is_finite() && v > 0.0 => args_out.look.exposure = v,
                    _ => return Err(format!("`--exposure {value}` — expected a positive number")),
                }
            }
            "--mcp" => args_out.mcp = Some(number_for("--mcp", "a port number", &mut it)?),
            "--tempo-source" => {
                args_out.tempo_source = Some(value_for("--tempo-source", &mut it)?);
            }
            "--audio-in" => {
                args_out.audio_in = Some(value_for("--audio-in", &mut it)?);
            }
            "--midi-in" => {
                args_out.midi_in = Some(value_for("--midi-in", &mut it)?);
            }
            "--midi-map" => {
                args_out.midi_map = Some(PathBuf::from(value_for("--midi-map", &mut it)?));
            }
            "--latency-offset-ms" => {
                let value = value_for("--latency-offset-ms", &mut it)?;
                // Refused rather than clamped, on the same terms as
                // `--exposure`: an offset silently changed is an offset the
                // operator will spend the first song chasing.
                match value.parse::<f32>() {
                    Ok(v) if v.is_finite() && audio::LATENCY_OFFSET_RANGE.contains(&v) => {
                        args_out.latency_offset_ms = v
                    }
                    _ => {
                        return Err(format!(
                            "`--latency-offset-ms {value}` — expected {} to {} milliseconds",
                            audio::LATENCY_OFFSET_RANGE.start(),
                            audio::LATENCY_OFFSET_RANGE.end()
                        ))
                    }
                }
            }
            "--watch" => args_out.watch = true,
            "--publish" => {
                let v = value_for("--publish", &mut it)?;
                args_out.published.push(parse_publish(&v)?);
            }
            "--merge" => {
                let v = value_for("--merge", &mut it)?;
                let slot: usize = v.parse().map_err(|_| {
                    format!("`--merge {v}` — expected a slot number, 0 for the first")
                })?;
                if !args_out.merge.contains(&slot) {
                    args_out.merge.push(slot);
                }
            }
            "--demo" => {
                let name = value_for("--demo", &mut it)?;
                args_out.demo = Some(Demo::from_name(&name).ok_or_else(|| {
                    format!("no demonstration named `{name}` — `transport` or `lines`")
                })?);
            }
            "--store" => args_out.store = PathBuf::from(value_for("--store", &mut it)?),
            // Read into a local rather than onto `Args`: a listing is not a
            // run, and the answer is returned below once the whole command line
            // has been seen — `--list-sets --store DIR` and `--store DIR
            // --list-sets` are the same request.
            "--list-sets" => list_sets = true,
            // Locals rather than `Args` fields, for `--list-sets`'s reason:
            // neither is a run, and both are answered below once the whole
            // command line has been seen, so `--store` may be on either side.
            "--package" => package = Some(value_for("--package", &mut it)?),
            "--take-in" => take_in = Some(PathBuf::from(value_for("--take-in", &mut it)?)),
            "--save-set" => args_out.save_set = Some(value_for("--save-set", &mut it)?),
            "--load-set" => args_out.load_set = Some(value_for("--load-set", &mut it)?),
            "--record-session" => {
                args_out.record_session = Some(value_for("--record-session", &mut it)?)
            }
            "--replay" => args_out.replay = Some(value_for("--replay", &mut it)?),
            "--budget-ms" => {
                args_out.budget_ms = number_for("--budget-ms", "a number of milliseconds", &mut it)?
            }
            "--frames" => args_out.frames = number_for("--frames", "a frame count", &mut it)?,
            "--capacity" => {
                args_out.capacity = number_for("--capacity", "an element count", &mut it)?;
                args_out.capacity_given = true;
            }
            "--size" => {
                args_out.size = extent("--size", value_for("--size", &mut it)?)?;
                size_given = true;
            }
            "--canvas" => {
                args_out.canvas = extent("--canvas", value_for("--canvas", &mut it)?)?;
                args_out.canvas_given = true;
            }
            // An unknown option used to become a path, so `--wtach` looked
            // like a `.kir` that did not exist and the error blamed the file.
            other if other.starts_with('-') => return Err(format!("unknown option `{other}`")),
            _ => positional.push(PathBuf::from(arg)),
        }
    }

    // **Answered before a word is said about material**, and above every rule
    // below: none of them is about a listing. A `.kir` pair this run will not
    // read, a default pair nobody asked for, a refusal about two ways of naming
    // material — all of it belongs to a run, and refusing `--list-sets
    // --load-set x` for "two descriptions of the material" would be a refusal
    // about a Set nothing here is going to build.
    if list_sets {
        return Ok(ParseOutcome::ListSets(args_out.store));
    }
    // The same, and above the material rules for the same reason. Answered in
    // a fixed order rather than refused as a pair: each of the three prints and
    // stops, so the only thing a second one could change is which answer is
    // printed, and none of them is a run whatever the other says.
    if let Some(id) = package {
        return Ok(ParseOutcome::Package {
            store: args_out.store,
            id,
        });
    }
    if let Some(file) = take_in {
        return Ok(ParseOutcome::TakeIn {
            store: args_out.store,
            file,
        });
    }

    // The bare positional pair still means what it always meant, and now it is
    // simply the last slot: `--set a,b c.kir d.kir` is a deck of two.
    match positional.len() {
        0 => {}
        2 => args_out.sets.push((
            Named::bare(positional[0].clone()),
            vec![Named::bare(positional[1].clone())],
        )),
        n => {
            return Err(format!(
                "{n} file argument(s) — a Set is an L1 and an L4, so give two, or use --set"
            ))
        }
    }
    // The default pair, for a run that named no material at all. **Not when a
    // Set file is loaded**: that file *is* the material, and adding the default
    // beside it would put a second Set on the deck nobody asked for — which is
    // not merely extra, it is a `--bind` from the file landing on material that
    // has no such parameter and saying so.
    if args_out.sets.is_empty() && args_out.load_set.is_none() {
        // A demonstration that needs a particular scene brings it, because one
        // that asks the operator to assemble it first is not a demonstration.
        // Only when nothing else named material: `--set` still wins.
        match args_out.demo.map(Demo::deck).filter(|d| !d.is_empty()) {
            Some(deck) => args_out.sets = deck,
            None => args_out.sets.push((
                Named::bare("examples/drift_shell.kir"),
                vec![Named::bare("examples/soft_points.kir")],
            )),
        }
    }
    // Refused rather than resolved: a Set file describes the material, and two
    // `.kir` paths describe the material, and a run given both has been told
    // two different things about what to play.
    if args_out.load_set.is_some() && !args_out.sets.is_empty() {
        return Err(
            "--load-set names the material and so do the `.kir` paths beside it; give one \
             or the other"
                .to_string(),
        );
    }
    if args_out.load_set.is_some() && args_out.save_set.is_some() {
        return Err(
            "--load-set and --save-set in one run: it would rewrite what it just read".to_string(),
        );
    }
    // An offscreen run is a function of its inputs — that is why it never
    // watches files either. Accepting `--audio-in` here and quietly ignoring it
    // would produce a PNG sequence whose bindings all sat at a tenth effect
    // with nothing to say why.
    if args_out.audio_in.is_some() && (args_out.render_to.is_some() || args_out.seq_to.is_some()) {
        return Err(
            "`--audio-in` with `--render` or `--seq` — an offscreen run takes no live input, \
             because its output has to be a function of its arguments. Drop one of them"
                .to_string(),
        );
    }
    // The same, and it is the same argument: a surface is a pair of hands, and
    // an offscreen render has nobody at it.
    if args_out.midi_in.is_some() && (args_out.render_to.is_some() || args_out.seq_to.is_some()) {
        return Err(
            "`--midi-in` with `--render` or `--seq` — an offscreen run takes no live input, \
             because its output has to be a function of its arguments. Drop one of them"
                .to_string(),
        );
    }
    let offscreen = args_out.render_to.is_some() || args_out.seq_to.is_some();
    // **`--mcp` with `--load-set` used to be refused here**, because a Set
    // built from the store had no procedure files on disk for a model to read
    // or rewrite. The scratch is where they go now: `--load-set` writes its two
    // procedures there like any other material, so a saved Set is editable and
    // the round trip — save, load, edit, save — closes. See `scratch::place`.
    // A surface with nobody at it, on the same terms as the other two — and
    // one more reason besides: an offscreen run is a function of its arguments,
    // and a port that can rewrite a procedure mid-render is the opposite of
    // that.
    if args_out.mcp.is_some() && (offscreen || args_out.replay.is_some()) {
        return Err(
            "`--mcp` with `--render`, `--seq` or `--replay` — an offscreen run takes no \
             live input, because its output has to be a function of its arguments"
                .to_string(),
        );
    }
    // Third of the same kind, and the same argument: a tempo source is another
    // machine's clock, and an offscreen run's output has to be a function of
    // its arguments. A replay has a stronger reason still — it follows the grid
    // the session recorded, so a live source would be overwriting the
    // performance it is supposed to be reproducing.
    if args_out.tempo_source.is_some() && (offscreen || args_out.replay.is_some()) {
        return Err(
            "`--tempo-source` with `--render`, `--seq` or `--replay` — an offscreen run \
             takes no live input, and a replay follows the grid the session recorded"
                .to_string(),
        );
    }
    // A map with no port is a file nothing reads, and the likely cause is a
    // forgotten `--midi-in` rather than a deliberate one.
    if args_out.midi_map.is_some() && args_out.midi_in.is_none() {
        return Err(
            "`--midi-map` with no `--midi-in` — there is no surface for the map to be of"
                .to_string(),
        );
    }
    // `--size` is the preview window's and an offscreen run has no window. It
    // used to be the render size too, and that is exactly the confusion being
    // removed: a reader who types `--render out.png --size 1920x1080` today
    // means `--canvas`, and quietly rendering at 1280x720 because `--size` no
    // longer reaches the canvas would be the worst of the three outcomes.
    if size_given && (offscreen || args_out.replay.is_some()) {
        return Err(
            "`--size` sets the preview window, and this run has no window. \
             Use `--canvas` for what is rendered"
                .to_string(),
        );
    }
    // **Not the argument the two above make.** `--record-session` is an output,
    // so "an offscreen run takes no live input" does not reach it. The reason is
    // that there is no performance here to record: an offscreen run advances one
    // step a frame and every edit it makes is a flag, so the stream would be
    // `--save-set`'s output followed by a constant — a file that looks like a
    // timeline and is a re-encoding of the command line.
    //
    // It was silently dropped instead, which is the failure the recorder's own
    // construction site has a comment warning against: a run that continued
    // without the recorder is "a performance nobody can replay and nothing
    // saying so". That guard fires when the *file* cannot be opened, and did
    // nothing when the flag never reached it at all — exit 0, no warning, no
    // session.
    if args_out.record_session.is_some() && (offscreen || args_out.replay.is_some()) {
        return Err(
            "`--record-session` with `--render`, `--seq` or `--replay` — there is no \
             performance to record: an offscreen run is a function of its arguments, and \
             the stream would say only what they already say"
                .to_string(),
        );
    }
    if args_out.sets.len() > MAX_SLOTS {
        return Err(format!(
            "{} Sets, and the deck holds {MAX_SLOTS}",
            args_out.sets.len()
        ));
    }
    Ok(ParseOutcome::Run(Box::new(args_out)))
}

/// What a replay renders at: the stream's, or the flag's, or a refusal — plus
/// a line to print when the answer is not simply what was performed.
///
/// **A function rather than four arms inside `replay_session`** for the reason
/// every decoder in this program is one: the decision needs a stream, a store
/// and a GPU to reach otherwise, and a decision nothing can test is a decision
/// that drifts. It is also the shape the refusal has to have — it cannot live
/// in `parse_args_from`, because at parse time nothing has read the stream.
fn replay_canvas(
    recorded: Option<(u32, u32)>,
    flag: (u32, u32),
    flag_given: bool,
) -> Result<((u32, u32), Option<String>), String> {
    match (recorded, flag_given) {
        // The stream knows, so the flag is refused rather than obeyed:
        // honouring it would render a session at a size it never ran at while
        // reporting a faithful replay.
        (Some(_), true) => Err(
            "`--canvas` with `--replay` — this session records what it rendered at, \
             and a replay is at that size or it is not a replay"
                .to_string(),
        ),
        (Some(size), false) => Ok((size, None)),
        // **The flag is the only way out for a stream that predates the record
        // or was written by hand.** Refusing it unconditionally forced every
        // such session to the default while saying "the session records what it
        // rendered at" about one that does not.
        (None, true) => Ok((
            flag,
            Some(format!(
                "no `canvas` record: replaying at the {}x{} `--canvas` asked for",
                flag.0, flag.1
            )),
        )),
        (None, false) => Ok((
            flag,
            Some(format!(
                "no `canvas` record and no `--canvas`: replaying at {}x{}, which is a \
                 guess rather than what was performed",
                flag.0, flag.1
            )),
        )),
    }
}

/// How many elements a Set gets: what was asked for, or what the procedure asks
/// for itself.
///
/// **A `.kir` declares `capacity [min, max] = default` and the default was never
/// used.** The range was enforced and the default silently lost to
/// `--capacity`'s own, so a procedure written for 131072 elements ran at 262144
/// unless somebody knew to say so — and an example whose point is visible only
/// at the count it was written for did not show its point. The same shape as
/// `--size` overriding a canvas: a general flag with a default beating a
/// specific declaration that meant it.
fn capacity_for(args: &Args, l1: &karakuri_ir::typed::Checked) -> u32 {
    if args.capacity_given {
        return args.capacity;
    }
    l1.capacity
        .map_or(args.capacity, |declared| declared.default)
}

/// The same question asked once per source, which is the only form the engine
/// accepts — `Set::build_many` takes `(procedure, capacity)` pairs precisely
/// because each source declares its own range.
///
/// **It was asked once and answered for everybody.** The build resolved
/// `capacity_for(args, &l1[0])` and handed that one number to every source, so
/// a second geometry ran at the first one's count with nothing printed: a grid
/// written for 512 x 256 samples and declared at 131072 drew 32768 of them
/// because it was loaded beside a cube. Nothing refused it either — the number
/// came from a declaration, so it was inside somebody's range, just not the
/// range of the procedure it was applied to.
///
/// `recorded` is what a Set file said, per geometry, and it wins where it said
/// anything: **the file is where that geometry's count was decided**, and a
/// Set that came back at a different size is a Set that was not saved. Empty
/// for a slot no file filled, which is every slot but slot 0.
fn capacities_for(
    args: &Args,
    l1s: &[karakuri_ir::typed::Checked],
    recorded: &[Option<u32>],
) -> Vec<u32> {
    l1s.iter()
        .enumerate()
        .map(|(at, l1)| {
            recorded
                .get(at)
                .copied()
                .flatten()
                .unwrap_or_else(|| capacity_for(args, l1))
        })
        .collect()
}

/// Refuse a canvas the GPU cannot make a texture of, by name.
///
/// **Not in `extent`**, because the number it is checked against is the
/// adapter's rather than the format's: `max_texture_dimension_2d` is 8192 on
/// some machines and 16384 on others, so a canvas is legal or not depending on
/// what is running the run. That makes it the earliest point *after* a device
/// exists rather than the latest point before one does.
///
/// The alternative is what happened before: a panic out of `create_texture`
/// naming a wgpu limit, from inside a call stack that says nothing about
/// `--canvas`. Every other refusal in this program names the flag.
fn check_canvas(device: &wgpu::Device, width: u32, height: u32) {
    let limit = device.limits().max_texture_dimension_2d;
    if width > limit || height > limit {
        eprintln!(
            "karakuri-cli: `--canvas {width}x{height}` — this GPU renders at most \
             {limit}x{limit}"
        );
        std::process::exit(1);
    }
}

/// Each slot's seed, derived from its index alone so that two slots given the
/// same pair are not the same picture twice — and so that the derivation is a
/// pure function of the deck layout rather than of anything measured. Slot 0
/// is [`SEED`] unchanged, so a one-Set run renders exactly what it always did.
fn seed_for(slot: usize) -> u32 {
    SEED.wrapping_add((slot as u32).wrapping_mul(0x9E37_79B9))
}

/// **What each of a slot's geometries is salted with**, in the order its L1
/// procedures were given — one number per geometry.
///
/// `recorded` is what a Set file said, per geometry, and it wins where it said
/// anything. That is the whole of what recording a salt buys over deriving one:
/// `docs/ir-spec.md` asks for a value assigned when a source is added and read
/// back from the stream forever after, so that reordering `--set` stops
/// changing which grid gets which randomness. Empty for a slot no file filled,
/// which is every slot but slot 0.
///
/// **Derived where nothing recorded one**, by the engine's own fallback rather
/// than by a formula spelled out a second time here — and the spec licenses
/// that squarely: *where it came from stops mattering once it is recorded*. A
/// bare `--set` run is salted by ordinal and looks exactly as it always did;
/// the moment `--save-set` writes these numbers down they stop being derived,
/// which is why this is also what the writer asks. One function, so the file
/// cannot record a salt the run was not using — the shape [`capacities_for`]
/// was fixed into after recording the flag's number and drawing another.
fn salts_for(seed: u32, recorded: &[Option<u32>], geometries: usize) -> Vec<u32> {
    (0..geometries)
        .map(|at| {
            recorded
                .get(at)
                .copied()
                .flatten()
                .unwrap_or_else(|| karakuri_engine::set::derived_salt(seed, at))
        })
        .collect()
}

/// **Render a recorded session.** The material comes from the stream's head and
/// every frame advances by the `tick` that was recorded, so nothing here reads
/// a clock — which is the whole claim: a replay and the run it came from are
/// the same sequence of frames.
///
/// Offscreen only. A window would add a clock back at the one place a replay
/// must not have one: `RedrawRequested` arrives when the display says so, and a
/// replay's frames belong to the stream.
/// **What to say about the records after the last tick.**
///
/// Said rather than dropped, on the same terms as everything else here: a
/// session that ended between frames recorded what the operator last did, and
/// nothing renders it because there is no frame it belongs to.
///
/// **A `save` among them is named**, which counting alone did not do. The rule
/// in `docs/ir-spec.md` is that a replay says *which* effects outside the stream
/// it skipped, and this path was obeying half of it: a `save` inside a frame was
/// named and a `save` after the last tick was folded into a number. That is the
/// wrong half to lose, because the last thing an operator does before quitting
/// is press `k` — a save at the end of a set lands here rather than inside a
/// frame, so the same key press was reported two different ways depending on
/// whether another frame followed it.
/// **What a replay says about a `save` it passed over**, in one place.
///
/// A save reaches a replay two ways — inside a frame, and after the last tick —
/// and each used to spell this sentence for itself. Two literals of one
/// sentence is the shape this codebase keeps removing, and the only test
/// reaching either of them asserted that the output contained "save" and the
/// id, so the two were free to drift apart without anything failing. `ir-spec`
/// requires that a replay say *which* effects outside the stream it skipped;
/// what it does not require is that the answer depend on where in the stream
/// the record sat.
fn skipped_save(slot: u8, id: &str) -> String {
    format!(
        "  a `save` of slot {slot} was skipped: a replay writes no Set files. \
         The material it named is set `{id}`"
    )
}

fn trailing_notes(trailing: &[Record]) -> Vec<String> {
    if trailing.is_empty() {
        return Vec::new();
    }
    let mut notes = vec![format!(
        "{} record{} after the last tick belong to no frame and are not replayed",
        trailing.len(),
        if trailing.len() == 1 { "" } else { "s" }
    )];
    notes.extend(trailing.iter().filter_map(|record| match record {
        Record::Save { slot, id } => Some(skipped_save(*slot, id)),
        _ => None,
    }));
    notes
}

fn replay_session(args: &Args, id: &str) {
    let store = open_store(args);
    let lines = match store.read_session(id) {
        Ok(lines) => lines,
        Err(e) => {
            eprintln!("karakuri-cli: session `{id}`: {e}");
            std::process::exit(1);
        }
    };
    let stream = session::split(lines);
    let loaded = match setfile::from_lines(&store, id, &stream.head) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("karakuri-cli: session `{id}`: {e}");
            std::process::exit(1);
        }
    };
    for note in &loaded.notes {
        eprintln!("  {note}");
    }
    for note in trailing_notes(&stream.trailing) {
        eprintln!("  {note}");
    }

    let Some(out) = args.render_to.clone().or(args.seq_to.clone()) else {
        eprintln!("karakuri-cli: --replay renders offscreen; give --render FILE or --seq DIR");
        std::process::exit(1);
    };
    let sequence = args.seq_to.is_some();
    // **`--seq` creates its directory, and a replay reaching it did not.**
    // `render::to_sequence` does this and `render::replay` goes straight to the
    // driven loop beside it, so `--replay --seq` into a directory that does not
    // exist failed on the first frame it tried to write — after rendering every
    // frame before it.
    if let Some(dir) = &args.seq_to {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("karakuri-cli: {}: {e}", dir.display());
            std::process::exit(1);
        }
    }
    let gpu = Gpu::headless().expect("no GPU");
    // **The stream's, not the flag's** — `--canvas` with `--replay` is refused
    // for this reason. A session written by this program always carries one;
    // the fallback is for a stream that predates the record or was written by
    // hand, and it is named rather than assumed because a replay at the wrong
    // size is a replay of different pixels.
    let (recorded_canvas, later_canvases) = stream.canvas();
    let (w, h) = match replay_canvas(recorded_canvas, args.canvas, args.canvas_given) {
        Ok((size, note)) => {
            if let Some(note) = note {
                eprintln!("  {note}");
            }
            size
        }
        Err(refusal) => {
            eprintln!("karakuri-cli: session `{id}`: {refusal}");
            std::process::exit(1);
        }
    };
    if later_canvases > 0 {
        eprintln!(
            "  {later_canvases} later `canvas` record{} ignored — the canvas is fixed for a run",
            if later_canvases == 1 { "" } else { "s" }
        );
    }

    // The size came out of the stream rather than off the command line, so this
    // can refuse a session recorded on a machine with a larger limit than the
    // one replaying it — which is the case a flag check could never have caught.
    check_canvas(&gpu.device, w, h);

    // **The file's own, per geometry, and derived only where it recorded
    // none.** A replay that re-derived them would be a replay of the material
    // in different colours, which is exactly the failure the seed record was
    // added to stop.
    let salts = salts_for(seed_for(0), &loaded.salts, loaded.l1s.len());
    // **Read once, and handed to both the Set and every rebuild the stream
    // asks for**, which is `build_deck`'s rule and `recorded_camera`'s reason.
    let layering = layering_for(args, 0, loaded.layering);
    let live = loaded.live;
    let mut set = build(
        &gpu,
        &loaded.l1s,
        // **The chain the file recorded**, which is the whole of what was
        // played: the deformers in chain order, the camera if it had one, the
        // field if it had one. A head that names none of them replays from the
        // built-in orbit, exactly as every session recorded before a Set file
        // could carry a chain does.
        &loaded.l2s,
        &loaded.l3s,
        &loaded.fields,
        &loaded.l4s,
        // **The head's own layering**, which it records — so a session whose
        // slot 0 was compositing replays compositing, and the `select` records
        // in the stream land on a fold that is there to be selected in. A head
        // that says nothing is a Set that overdrew, which is what saying
        // nothing has always meant; `--merge 0` beside a replay still turns it
        // on, on `layering_for`'s terms, which is what lets a session recorded
        // before the record existed be replayed as it was played.
        layering,
        // **The names and edges the file recorded.** A replay is the material
        // as it was played, and an edge is part of the material: a slot the
        // file bound and a replay did not would be a Set that will not build,
        // and one bound to a different geometry is a different picture.
        &loaded.names,
        &loaded.edges,
        // The file's number per geometry when it recorded one, and otherwise
        // the procedure's own declared default — never the flag's, which is a
        // general default beating a specific declaration that meant it.
        &capacities_for(args, &loaded.l1s, &loaded.capacities),
        &mut vec![false; loaded.bindings.len()],
        &loaded.params,
        &loaded.bindings,
        // **A Set file does not record an interface yet**, on the same terms it
        // records neither a chain nor a camera: it names an L1, its renderers,
        // their values and their bindings. Publishing nothing is
        // publishing everything, so a replay shows the whole console — which is
        // the safe direction, since an interface is about attention.
        &[],
        // The Set's own seed — what an L3 reads — is the first geometry's,
        // which is what one number can hold and what a file recording one seed
        // has always meant by it.
        salts.first().copied().unwrap_or_else(|| seed_for(0)),
        &salts,
        loaded.camera,
        // **The fold the head was left with.** A `select` record later in the
        // stream moves it, exactly as it did live; this is where the run
        // started, and a replay that began with every renderer live would be a
        // replay of a different first frame.
        live,
    );
    set.resize(&gpu.device, w, h);
    // **A deck of one, from one Set file**, which is what a replay can build
    // today: a session stream names the slots its records act on but carries no
    // way to say what was in them, since a Set file describes one Set and there
    // is no record for "this deck held these four". So a record naming slot 1
    // or beyond is reported and skipped — see `apply_replayed` — and a session
    // recorded on a full deck replays the first slot's performance rather than
    // the deck's. That is a hole in the *stream's* vocabulary rather than in
    // this function, and it is the same hole `gain`, `blend`, `residency` and
    // `preview` all fall into.
    let mut deck = Deck::new(&gpu.device, vec![HotSwap::fixed(set)], w, h);
    deck.set_signals(Signals::new(args.bpm, u64::from(SEED)));

    let frames = u32::try_from(stream.frames.len()).unwrap_or(u32::MAX);
    eprintln!(
        "replaying session `{id}`: {frames} frames, {w}x{h}, {} at exposure {:.2} -> {}",
        op_name(args.look.op),
        args.look.exposure,
        out.display()
    );

    // What each slot is playing, as the stream names it. Seeded from nothing:
    // the head already built the Sets a run started with, so the first
    // `procedure` record is the first *change*.
    // The L1, and the renderers by index — a slot draws with one geometry and
    // however many L4s, so the second half is a list rather than a slot.
    let mut playing: Vec<(
        Option<karakuri_store::hash::Hash>,
        Vec<Option<karakuri_store::hash::Hash>>,
    )> = vec![(None, Vec::new()); deck.slot_count()];
    // A `look` record in the stream moves this, so it is **returned** from the
    // driver each frame rather than handed to the renderer once before the run.
    // It used to be both, and the parameter is the one that won: a session where
    // the operator changed the tone mapper or the exposure replayed under
    // whatever it started with, however many `look` records the stream carried.
    // The `--look` and `--exposure` flags are only the value it starts at, on
    // the same terms as a live run.
    let mut look = args.look;
    let result = render::replay(
        &gpu,
        &mut deck,
        w,
        h,
        frames,
        |i| {
            if sequence {
                Some(out.join(format!("{i:05}.png")))
            } else if i + 1 == frames {
                Some(out.clone())
            } else {
                None
            }
        },
        |i, deck| {
            let frame = &stream.frames[i as usize];
            // **Procedures are gathered and applied together, after the rest.**
            // A slot's procedures arrive as a group and a Set is built from all
            // of them — rebuilding on the first would compile an L1 against the
            // L4 it is replacing, which is a composition the performance never
            // had and may not even check.
            //
            // **A slot's renderer list is replaced, not merged.** A rebuild
            // restates the whole stack, so the records in this frame are the
            // whole stack; keeping what was there would leave a stale renderer
            // behind whenever a slot went from three to one, and nothing in the
            // vocabulary can say "one fewer" on its own.
            let mut changed: Vec<usize> = Vec::new();
            let mut restated: Vec<usize> = Vec::new();
            for record in &frame.before {
                if let karakuri_store::record::Record::Procedure {
                    slot,
                    layer,
                    index,
                    proc_hash,
                } = record
                {
                    let slot = usize::from(*slot);
                    if slot >= playing.len() {
                        eprintln!(
                            "  a `procedure` for slot {slot} was skipped: this replay \
                             builds a deck of one"
                        );
                        continue;
                    }
                    match layer {
                        Layer::L1 => playing[slot].0 = Some(*proc_hash),
                        // **Indexed, and the list grows to fit.** A stack's
                        // renderers arrive as one `procedure` record each,
                        // numbered in draw order; a stream from before stacks
                        // existed carries index 0 and lands in the same place
                        // the old code put it.
                        Layer::L4 => {
                            // The first L4 record for this slot in this frame
                            // starts the list over — see "replaced, not merged"
                            // above.
                            if !restated.contains(&slot) {
                                restated.push(slot);
                                playing[slot].1.clear();
                            }
                            let at = *index as usize;
                            if playing[slot].1.len() <= at {
                                playing[slot].1.resize(at + 1, None);
                            }
                            playing[slot].1[at] = Some(*proc_hash);
                        }
                        other => {
                            eprintln!("  a `procedure` for {other:?} was skipped");
                            continue;
                        }
                    }
                    if !changed.contains(&slot) {
                        changed.push(slot);
                    }
                    continue;
                }
                // **A replay is a sandbox, and this is the record that makes
                // that a rule rather than a description.** Every other record
                // here describes the deck, and obeying it is what replaying
                // means; a `save` describes a file in a store, and obeying it
                // would mean writing into an id that already exists in a
                // library nobody asked this run to touch — so `--replay` would
                // stop being a function from a stream to some frames.
                //
                // **Said, not silently dropped.** `load_set` prints every note
                // it could not honour and the session writer counts the batches
                // it lost; a replay that skipped an outside effect in silence
                // would be the one place in this program where something
                // happened and nothing said so. The id is named because it is
                // what an operator would go and load by hand.
                if let karakuri_store::record::Record::Save { slot, id } = record {
                    eprintln!("{}", skipped_save(*slot, id));
                    continue;
                }
                apply_replayed(deck, &mut look, record);
            }
            for slot in changed {
                match rebuild(&gpu, &store, &playing[slot], args, slot, layering, live) {
                    Ok(set) => deck.install(&gpu.device, slot, set),
                    Err(e) => eprintln!("  slot {slot}: {e} — it keeps what it had"),
                }
            }
            (frame.steps, look)
        },
    );
    if let Err(e) = result {
        eprintln!("karakuri-cli: {e}");
        std::process::exit(1);
    }
}

/// Build the Set a slot's `procedure` records name.
///
/// **Every node or nothing.** A `procedure` record names one node, and a Set is
/// built from all of them — so until the L1 and every renderer have been seen
/// there is nothing to build, and a slot whose stream only ever names some of
/// them keeps what the head gave it. That is not a corner case: the writer emits
/// the whole stack, but a stream from a future version might name only what
/// changed.
///
/// A **gap** in the renderer list is refused on the same terms rather than
/// closed up: index 2 without index 1 describes a stack with a hole in it, and
/// silently shifting the third renderer into second place would change draw
/// order.
fn rebuild(
    gpu: &Gpu,
    store: &karakuri_store::store::Store,
    playing: &(
        Option<karakuri_store::hash::Hash>,
        Vec<Option<karakuri_store::hash::Hash>>,
    ),
    args: &Args,
    slot: usize,
    // **The layering and the selection the head was built at**, restated here
    // rather than re-derived — `watch::Watch::layering` states the rule and
    // this is the replay's copy of the same hazard: a `procedure` record names
    // a swapped-in procedure and says nothing about how the slot's renderers
    // meet, so a rebuild that worked it out again would put a composited
    // session onto overdraw at the first swap, with a `select` landing on a
    // fold that is no longer there.
    //
    // **A later `select` in the stream moves the selection and this does not
    // know it**, which is the same thing a live rebuild does to a slot the
    // operator pressed `r` on. It is the head's fold, restated; a stream that
    // selected again after the swap selects again on replay.
    layering: karakuri_engine::set::Layering,
    live: Option<u32>,
) -> Result<Set, String> {
    let (Some(l1_hash), l4_hashes) = playing else {
        return Err("its L1 has not been named".into());
    };
    if l4_hashes.is_empty() || l4_hashes.iter().any(Option::is_none) {
        return Err("not every one of its renderers has been named".into());
    }
    let source = |hash: &karakuri_store::hash::Hash| -> Result<String, String> {
        let bytes = store
            .get_artifact(hash)
            .map_err(|e| format!("reading `{hash}`: {e}"))?;
        String::from_utf8(bytes).map_err(|e| format!("`{hash}` is not text: {e}"))
    };
    // Checked again rather than trusted. The store is content-addressed, so
    // these are the exact bytes that compiled during the performance — but a
    // build this program can refuse is a build it must refuse, and the
    // diagnostics belong on the terminal either way.
    let l1 = compile::check(&source(l1_hash)?).map_err(|report| format!("L1:\n{report}"))?;
    let l4s = l4_hashes
        .iter()
        .flatten()
        .map(|h| compile::check(&source(h)?).map_err(|report| format!("L4:\n{report}")))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(build(
        gpu,
        std::slice::from_ref(&l1),
        // Artifacts recorded by a session, which stores an L1 and its
        // renderers — see the note at the other `build` call site.
        &[],
        &[],
        &[],
        &l4s,
        layering,
        &Names::default(),
        // **No node here declares a slot**, because there is no L2 here at all:
        // a `procedure` record names an L1 and its renderers.
        &[],
        // Nothing recorded: a `procedure` record names a swapped-in procedure
        // and carries no capacity, so each source runs at what it declares.
        &capacities_for(args, std::slice::from_ref(&l1), &[]),
        &mut vec![false; args.bindings.len()],
        &args.overrides,
        &args.bindings,
        &[],
        seed_for(slot),
        // **Nothing recorded, on the same terms as the capacity above.** A
        // `procedure` record names a swapped-in procedure and carries no salt,
        // so the one source is salted from the slot's seed and its ordinal.
        &salts_for(seed_for(slot), &[], 1),
        None,
        live,
    ))
}

/// One record from a session, applied to a replaying deck.
///
/// Deliberately the same decoders the live path uses — `mix::change` and
/// `audio::apply_tempo` — because that is the whole point of the arrangement:
/// what drove the engine live and what drives it on replay are the same
/// function, so they cannot come apart.
fn apply_replayed(deck: &mut Deck, look: &mut Look, record: &karakuri_store::record::Record) {
    // The two the signal bus takes, through the same decoders the live path
    // uses. `audio` is what makes a replay reproduce what the room sounded
    // like: without it a binding to `energy` would replay at the confidence
    // the bus invents rather than at what a microphone heard.
    match record {
        karakuri_store::record::Record::Tempo { .. } => {
            let mut signals = *deck.signals();
            audio::apply_tempo(&mut signals, record);
            deck.set_signals(signals);
            return;
        }
        karakuri_store::record::Record::Audio { .. } => {
            let mut signals = *deck.signals();
            signals.set_audio(audio::audio_frame(record));
            deck.set_signals(signals);
            return;
        }
        _ => {}
    }
    match mix::change(record, deck.slot_count()) {
        Ok(Some(mix::Change::Gain { slot, value })) => deck.set_gain(slot, value),
        Ok(Some(mix::Change::Opacity { slot, value })) => deck.set_opacity(slot, value),
        Ok(Some(mix::Change::Blend { slot, mode })) => deck.set_blend(slot, mode),
        Ok(Some(mix::Change::Mask { slot, mask })) => deck.set_mask(slot, mask),
        Ok(Some(mix::Change::Transition {
            slot,
            control,
            to,
            start,
            beats,
            curve,
        })) => deck.schedule(schedule_from(deck, slot, control, to, start, beats, curve)),
        Ok(Some(mix::Change::Select {
            slot,
            renderer,
            start,
        })) => {
            // The same check the live path makes, against the same fact, and
            // this is the path it is actually likely on: a session replayed
            // against material that has since lost a renderer.
            let count = deck.slot(slot).set().inputs().len();
            match renderer_in_range(slot, renderer, count) {
                Ok(()) => deck.schedule_selection(karakuri_engine::transition::Selection::new(
                    slot, renderer, start,
                )),
                Err(refusal) => eprintln!("  {refusal} — skipped"),
            }
        }
        // **Governed, exactly as the live path governs.** `set_residency`
        // writes the request *and* grants it, and only a governor pass
        // re-derives what the deck is actually doing against the budget of the
        // machine it is on. A replay used to skip the pass entirely, so every
        // request was granted — which is precisely what `Record::Residency`'s
        // documentation says must not happen, since it records the request and
        // never the effective level for the reason that a session recorded on a
        // fast machine and replayed on a slow one has to re-derive it.
        //
        // Latent until a session can carry a deck: a replay builds one slot, and
        // one slot does not exhaust a budget. Closed anyway, because the reason
        // it was invisible is that the two paths were different code.
        Ok(Some(mix::Change::Residency { slot, level })) => {
            deck.set_residency(slot, level);
            report_governing(&deck.govern(), "residency");
        }
        Ok(Some(mix::Change::Look(l))) => *look = l,
        Ok(Some(mix::Change::Transport {
            slot,
            sync,
            anchor_bpm,
            scrub_beats,
        })) => {
            if let Err(refusal) = deck.set_transport(slot, sync, anchor_bpm, scrub_beats) {
                eprintln!("  slot {slot}: {} sync refused — {refusal}", sync.name());
            }
        }
        Ok(None) => {}
        Err(message) => eprintln!("  {message} — skipped"),
    }
}

/// Build a scheduled move out of a decoded record and the value the control is
/// at **now**.
///
/// The `from` end is read here rather than carried in the record, which is the
/// whole of why this function exists and is shared by the live path and the
/// replay path: both have to read it at the same point in the stream or a
/// replay would fade from somewhere the run did not. `session::split` puts a
/// key press between two ticks into that frame's `before` list, which is the
/// position it was applied at live, so they do.
fn schedule_from(
    deck: &Deck,
    slot: usize,
    control: karakuri_engine::transition::Control,
    to: f32,
    start: f64,
    beats: f64,
    curve: Curve,
) -> karakuri_engine::Transition {
    use karakuri_engine::transition::Control;
    let from = match control {
        Control::Gain => deck.gain(slot),
        Control::Opacity => deck.opacity(slot),
        Control::MaskPosition => deck.mask(slot).position(),
    };
    karakuri_engine::Transition::new(slot, control, from, to, start, beats, curve)
}

/// Open the store, or stop with the reason. Both directions need one and
/// neither can do anything useful without it.
fn open_store(args: &Args) -> karakuri_store::store::Store {
    match karakuri_store::store::Store::open(&args.store) {
        Ok(store) => store,
        Err(e) => {
            eprintln!("karakuri-cli: store `{}`: {e}", args.store.display());
            std::process::exit(1);
        }
    }
}

/// **What the store holds, one line per Set** — the operator's half of what
/// `list_sets` tells a model.
///
/// **It prints and stops.** No window, no adapter, no compile, no Set built:
/// asking what is in a library is not a run, and a flag that opened a GPU to
/// answer it would be unusable over ssh on the machine the library is on. That
/// is why it is answered out of [`parse_args_from`] — see [`ParseOutcome`] —
/// rather than somewhere down `main` where it would have to be kept above every
/// early return by hand.
///
/// **The same summary the MCP tool renders**, from
/// [`setfile::summarise`]: one derivation, two renderings. What a node
/// is called here is what `read_set` calls it, because the answer comes from
/// one function — an operator reading a line here and a model reading a block
/// there are looking at one library and must be told one thing about it.
///
/// A compact line and not a block: the question is *which of these do I want*,
/// and what answers it is the id to type next to `--load-set`, when it was
/// saved, and enough of what it holds to tell two of them apart. What each node
/// declares is `read_set`'s answer, over MCP, on one Set at a time.
fn listed_sets(
    store: &karakuri_store::store::Store,
    root: &std::path::Path,
) -> Result<String, String> {
    let mut sets =
        setfile::summarise(store).map_err(|e| format!("store `{}`: {e}", root.display()))?;
    if sets.is_empty() {
        // Not an error and not silence: an empty store is what a store looks
        // like before anything has been kept in it, and the answer says where
        // sets come from rather than leaving a blank terminal to be read as a
        // failure.
        return Ok(format!(
            "no sets in `{}` — nothing has been kept here yet. `--save-set ID` writes \
             one, and so does the `k` key during a run.\n",
            root.display()
        ));
    }
    // **Most recent first, breaking ties by id.** `--save-set` twice in one
    // second gives two files one mtime on a coarse filesystem clock, and a sort
    // whose keys tie leaves the order to whatever `read_dir` said — so two runs
    // of this flag over an untouched store would print two different lists. The
    // id is unique by construction, which makes the order total.
    sets.sort_by(|a, b| b.written.cmp(&a.written).then_with(|| a.id.cmp(&b.id)));
    let width = sets.iter().map(|set| set.id.len()).max().unwrap_or(0);
    let mut out = String::new();
    for set in &sets {
        let _ = write!(
            out,
            "{:<width$}  {}  ",
            set.id,
            setfile::written_at(set.written)
        );
        match &set.unreadable {
            // Listed and named rather than dropped: a file in `sets/` that will
            // not read is the one thing here an operator has to go and look at.
            Some(why) => {
                let _ = writeln!(out, "unreadable: {why}");
            }
            None if set.nodes.is_empty() => {
                let _ = writeln!(out, "no material: it holds no `slot` record");
            }
            None => {
                let _ = writeln!(out, "{}", holdings(&set.nodes));
            }
        }
    }
    Ok(out)
}

/// The listing for a store path — **opening nothing that is not there.**
///
/// `Store::open` establishes the layout under a root that does not exist yet,
/// which is right for a run about to write into it and wrong here: a flag whose
/// whole promise is that it only reads must not leave a directory behind to say
/// that a library is empty. A path with no store at it is an answer, and it is
/// a different one from a store with no sets in it — the first is very often a
/// mistyped `--store`.
fn listed_sets_at(root: &std::path::Path) -> Result<String, String> {
    if !root.exists() {
        return Ok(format!(
            "no store at `{}` — nothing has ever been kept there, and nothing was \
             created to find that out. Check `--store`, or keep something with \
             `--save-set ID`.\n",
            root.display()
        ));
    }
    let store = karakuri_store::store::Store::open(root)
        .map_err(|e| format!("store `{}`: {e}", root.display()))?;
    listed_sets(&store, root)
}

/// **One Set, with every source it names inlined, as the text to print.**
///
/// **A package goes to standard output**, which is what this returning a
/// `String` is: it is a file you *send somebody* — one self-contained patch
/// that loads in a store which has never held the material — so it belongs
/// where `karakuri-cli --package night01 > night01.kbset` puts it. A
/// store directory would need a naming rule of its own for it, and a second
/// copy of a Set sitting beside the Set is a second answer to which of them is
/// the file.
///
/// **Opening nothing that is not there**, on [`listed_sets_at`]'s terms: a flag
/// that only reads must not leave a store behind to report that a Set is
/// missing from it. Here it is an error rather than an answer, because a bundle
/// of a Set that does not exist is not a bundle.
///
/// **Or an authoring file, and that is one flag rather than two.** ADR-0229
/// part 4 settles that packaging is *"one operation, two moments"* — loading an
/// authoring file *is* packaging it, and packaging for distribution is the same
/// resolution done ahead of time — so the flag grew the other moment instead
/// of a second flag beside it, and `docs/manual/operations.html` keeps the one
/// row it always had. And it is called packaging because of exactly that: the
/// half that takes a `.kset` resolves, stores and writes, so an export is only
/// one of the two things this flag does — see [`ParseOutcome::Package`].
/// **Which of the two is decided by the extension**, on
/// exactly the terms ADR-0231 made it load-bearing for: a value ending in
/// `.kset` is a path to an authoring file, and anything else is an id in the
/// store. Nothing else could decide it — an id and a relative path are both
/// bare words — and this is the same sentence the store already reads a name
/// with.
///
/// **The authoring half opens a store that is not there, where the id half
/// refuses to.** Resolving is a *write*: each part is read from disk, hashed
/// and put in the store as an artifact, so the store has to exist by the time
/// the first one lands, and establishing the layout under a root an operator
/// named is what every other writing path here does — see [`taken_in_file`].
/// The id half is still a pure read and still leaves nothing behind.
fn packaged_set(root: &std::path::Path, named: &str) -> Result<String, String> {
    let lines = if named.ends_with(setfile::AUTHORING_SUFFIX) {
        let store = karakuri_store::store::Store::open(root)
            .map_err(|e| format!("store `{}`: {e}", root.display()))?;
        setfile::bundle_authored(&store, std::path::Path::new(named))?
    } else {
        if !root.exists() {
            return Err(format!(
                "no store at `{}`, so there is no set `{named}` to package — check `--store`",
                root.display()
            ));
        }
        let store = karakuri_store::store::Store::open(root)
            .map_err(|e| format!("store `{}`: {e}", root.display()))?;
        setfile::bundle(&store, named)?
    };
    Ok(lines
        .iter()
        .map(|line| format!("{}\n", line.as_str()))
        .collect())
}

/// **A Set file somebody sent you, into this store**, and the report of what
/// happened.
///
/// **Both of a Set's forms, and the extension is the whole of what tells them
/// apart** — [`packaged_set`]'s rule, read from the other end, and ADR-0231's.
/// A `.kbset` is already resolved and carries its own sources, so it is read and
/// taken in as it stands. A `.kset` is the authoring form: it names its parts by
/// relative path, so it goes through `setfile::resolve` first — behind the same
/// wall, refusing the same escapes, absolute paths, `..` and symlinks alike.
///
/// **Resolved *and* inlined rather than resolved alone**, which is
/// `setfile::bundle_authored`, so that taking a `.kset` in lands the same store
/// as packaging it and taking the result in. `setfile::unbundle` writes a
/// metadata card for each source the lines carry; handing it resolved lines with
/// nothing inlined would file the Set and leave every artifact cardless — one
/// operation reaching two different stores by two routes, which is the
/// disagreement a second spelling always is.
///
/// The store *is* opened where it is not there, unlike [`packaged_set`]'s id
/// half: this is a write, and establishing the layout under a root an operator
/// named is what every other writing path here does.
fn taken_in_file(root: &std::path::Path, file: &std::path::Path) -> Result<String, String> {
    let store = karakuri_store::store::Store::open(root)
        .map_err(|e| format!("store `{}`: {e}", root.display()))?;
    let named = file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let lines = if named.ends_with(setfile::AUTHORING_SUFFIX) {
        setfile::bundle_authored(&store, file)?
    } else {
        karakuri_store::ndjson::read(file)
            .map_err(|e| format!("reading `{}`: {e}", file.display()))?
    };
    setfile::unbundle(&store, &lines)
}

/// What a Set holds, by layer and in the order the layers compose: `2 L1, 1 L2,
/// 3 L4`. A layer nothing is on is left out rather than printed as a zero,
/// because most Sets are on three of the five and a line of zeroes reads as
/// something missing.
fn holdings(nodes: &[setfile::NodeSummary]) -> String {
    [Layer::L1, Layer::L2, Layer::L3, Layer::L4, Layer::Field]
        .iter()
        .filter_map(|layer| {
            let n = nodes.iter().filter(|node| node.layer == *layer).count();
            (n > 0).then(|| format!("{n} {}", setfile::layer_name(*layer)))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Write the material as a Set file, and say where it went.
///
/// The first `--set` pair only. A Set file describes **one Set**, and a deck of
/// four is a session's arrangement rather than a Set's — that is the same line
/// `Record::is_set_state` draws, seen from the writing side.
/// **The material a session carries at its head**, so that a replay can build
/// what the run was playing.
///
/// This used to be the `--load-set` file or nothing, and "or nothing" was a
/// hole in the invariant the whole record stream exists for: recording without
/// `--load-set` wrote a timeline of ticks with no material under it, and
/// `--replay` refused it with "has no L1 slot" long after the set was over.
/// Nothing said so at the time.
///
/// So the head is written from **what the run is actually playing**. Without a
/// Set file there is one to make: the `.kir` pair, the capacity, the params,
/// the bindings and the seed are exactly what `--save-set` writes, and putting
/// the sources in the store is what makes the head's hashes resolve on the way
/// back. The Set file it leaves behind is named after the session, so a
/// recording is also a saved Set and neither had to be asked for twice.
///
/// **Only slot 0's material, and it says so**, which is the limitation
/// underneath rather than a choice made here: a Set file describes one Set and
/// a session stream has no way to say what a *deck* held. Everything else about
/// the performance is recorded per slot — gain, blend, residency, preview — so
/// a multi-slot session replays those against a deck of one and reports the
/// rest. Closing it is a format change, and it is named in `docs/ir-spec.md`
/// where the records are.
fn session_head(
    args: &Args,
    placed: &[Vec<Placed>],
    l1s: &[karakuri_ir::typed::Checked],
    store: &karakuri_store::store::Store,
    id: &str,
) -> Vec<karakuri_store::ndjson::Line> {
    if placed.len() > 1 {
        eprintln!(
            "  only slot 0's material is in the session's head — a session stream cannot \
             say what a deck held, so the other {} will not replay",
            placed.len() - 1
        );
    }
    if let Some(set) = &args.load_set {
        return match store.read_set(set) {
            Ok(lines) => lines,
            Err(e) => {
                eprintln!("karakuri-cli: reading set `{set}` for the session's head: {e}");
                std::process::exit(2);
            }
        };
    }
    let Some(nodes) = placed.first().filter(|nodes| !nodes.is_empty()) else {
        eprintln!("karakuri-cli: nothing to record — no Set to put at the session's head");
        std::process::exit(2);
    };
    let material = format!("{id}-material");
    let camera = karakuri_engine::camera::Orbit::default();
    // **Every source into the store before the file that references them.**
    // The writer takes hashes now — see `setfile::Node` — and this is the
    // caller whose paths are still exactly what the run compiled a moment ago.
    let nodes = match saving_nodes(store, nodes) {
        Ok(nodes) => nodes,
        Err(e) => {
            eprintln!("karakuri-cli: writing the session's material: {e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = setfile::save(
        store,
        // **The operator's**: this is the material of a run they started, kept
        // where they will look for it.
        Asked::Operator,
        &material,
        setfile::Saving {
            nodes: &nodes,
            capacities: &saving_capacities(args, l1s),
            params: &args.overrides,
            bindings: &args.bindings,
            // **Which node fills each declared slot**, saved so the file
            // rebuilds: a slot nothing binds is refused where the Set is built,
            // so a Set file that dropped its edges would be one that no longer
            // loads.
            edges: &args.edges,
            camera: &camera,
            // **The flags', on the terms every other value here is theirs.**
            // This head is written before the first frame, from the material
            // the run was started with — there is no Set to read a layering
            // off yet — and a session whose slot 0 composites has to say so or
            // the `select` records it goes on to write land on a replay with
            // no fold to select in. The other branch above needs none of this:
            // a run started from `--load-set` copies that file's lines
            // verbatim, `merge` among them.
            layering: layering_for(args, 0, recorded_layering(args, 0)),
            // **Nothing yet, and it is not an omission.** A selection is made
            // with `r` during a performance, so at the instant a head is
            // written there is none — and one made later is a `select` record
            // in the stream, which replays where it happened rather than
            // before the first frame.
            live: None,
            seeds: &saving_seeds(args, l1s),
        },
    ) {
        // Fatal, on the same terms the recorder itself is: `--record-session`
        // was asked for, and a run that continued would be a performance
        // nobody can replay with nothing saying so.
        eprintln!("karakuri-cli: writing the session's material: {e}");
        std::process::exit(2);
    }
    match store.read_set(&material) {
        Ok(lines) => lines,
        Err(e) => {
            eprintln!("karakuri-cli: reading back the session's material: {e}");
            std::process::exit(2);
        }
    }
}

/// **Every slot's launch sources into the store, because this run records one.**
///
/// A rollback onto the launch version writes `procedure` records naming those
/// hashes — see [`Running::rolled_back`] — and a replay meeting them resolves
/// each by reading the artifact back. A record naming bytes nobody kept is the
/// same silence as no record at all, so a recorded run owes the store those
/// bytes before the first frame.
///
/// **And only a recorded run owes them**, which is the judgement this function
/// exists to hold. The seeding used to happen for every windowed run on exactly
/// this reasoning, and the reasoning does not reach that far: a run with no
/// recorder names no hash anywhere outside itself, and a save that does name
/// one puts its own bytes as it writes the file — see [`Sources::into_nodes`].
/// What the wider version cost was a `.karakuri` directory created by a plain
/// windowed run, which `docs/manual.md` promises does not happen.
///
/// [`session_head`] has already put slot 0's material here on its way past;
/// `put_artifact` is content-addressed, so this repeats nothing and exists for
/// the other slots, which a session head cannot describe but a `procedure`
/// record can still name.
///
/// **Reported and not fatal.** The recorder itself is fatal on failure because
/// a run that continued would be a performance nobody can replay with nothing
/// saying so; this is narrower — one slot's *rollback* would be unresolvable —
/// and the sentence is the saying.
fn seed_store_for_replay(store: &karakuri_store::store::Store, placed: &[Vec<Placed>]) {
    for (slot, nodes) in placed.iter().enumerate() {
        for node in nodes {
            if let Err(e) = node.put(store) {
                eprintln!(
                    "  slot {slot}: {e} — a rollback of this slot onto the version it \
                     launched with will name a source this session's replay cannot resolve"
                );
            }
        }
    }
}

/// **Where one slot's sources come from at save time**: the hashes of what it is
/// running, and nothing else.
///
/// A slot is running a version whose bytes need not be on disk under any name.
/// A build rolled back for costing too much leaves a *newer* `.kir` behind it,
/// an edit that fails to compile stays on disk untouched, and a run without
/// `--watch` never picks a file up at all — so the path and the picture can
/// disagree in three ordinary ways, and in each of them a save that re-read the
/// path would write down a version nobody had seen. The hash is what still
/// points at what is on screen, which is why it is the *only* thing this reads
/// and why every slot has one from launch — see [`Running::at_launch`]. The
/// bytes behind a launch hash travel with it, because on a run that has saved
/// nothing they exist nowhere else; see [`SavedNode::source`].
///
/// `None` is a slot with no address to name: one filled straight from a Set file
/// with nothing to watch it, or one that has just taken a build whose sources
/// could not be stored and said so at the time. It saves nothing rather than
/// guessing, and `Live::save_set` says which it was.
///
/// **The operator's names are zipped on by position.** Both lists are in the
/// order the files were spelled — `sort_slot` keeps that deliberately and the
/// watcher zips its hashes onto the same list — so entry `n` of one is entry
/// `n` of the other. A name cannot come from the hashes: it belongs to the
/// *use* rather than to the procedure, so nothing a `procedure` record carries
/// could hold it.
///
/// A free function rather than a method, so that the choice — which is the
/// whole of what a live save gets right or wrong about what is on screen — can
/// be checked without a window and a GPU.
fn live_sources(playing: Option<&Nodes>, startup: &[Placed]) -> Sources {
    Sources(playing.map_or_else(Vec::new, |nodes| {
        nodes
            .iter()
            .enumerate()
            .map(|(at, (layer, index, hash))| {
                let placed = startup.get(at);
                // **Carried only where the address says these are the bytes on
                // screen.** Equal hashes mean the slot is still running what it
                // launched with at this node, so the compiled text this process
                // is holding is what the file will reference and the store has
                // to be given it. Unequal means a build put that version there,
                // and the watcher stored it as it built it — there is nothing
                // here to add.
                //
                // **One predicate asked once, yielding the pair.** The card and
                // the bytes travel on exactly the same condition, and
                // `SavedNode::meta` states that as an invariant —
                // `Sources::into_nodes` writes the card inside the `if let` for
                // the source and would silently drop a card that outlived its
                // bytes. Asked twice it was two derivations of one question with
                // nothing holding them together, which is the defect this file
                // has already paid for in `Running` and in `Sources`.
                let (source, meta) = placed
                    .filter(|p| p.hash() == *hash)
                    .map(|p| {
                        (
                            std::sync::Arc::clone(&p.source),
                            std::sync::Arc::clone(&p.meta),
                        )
                    })
                    .unzip();
                SavedNode {
                    layer,
                    index: *index,
                    hash: *hash,
                    name: placed.and_then(|p| p.named.name.clone()),
                    source,
                    meta,
                }
            })
            .collect()
    }))
}

/// **What every slot is running, and what a rollback would bring back.**
///
/// One representation of "what bytes is this node running", held per slot as the
/// addresses a `procedure` record names — so a slot holding a chain and two
/// geometries has a line for each rather than an L1 and some renderers.
///
/// **It is seeded before the first frame**, which is the property [`Sources`]
/// leans on: every slot with files behind it has a hash from the outset. It was
/// previously seeded from nothing and filled in only by the watcher, so a slot
/// nothing had rebuilt had no hash anywhere and the live saver reached for the
/// *paths* instead. That gave the question two answers, and they disagree in
/// every state where something has rewritten a file the run is not drawing
/// from. A slot that is still `None` here is one with no files behind it at
/// all — see [`Running::at_launch`] — and it saves nothing rather than guessing.
///
/// **A type of its own rather than two fields on `Live`**, because the pair is
/// one fact with one transition rule: a swap moves `playing` into `previous`, a
/// rollback moves it back, and the two halves are never right apart. It is also
/// what lets the transition be tested — the whole of it happens without a
/// window, a GPU or a governor.
///
/// **And the whole rule is in here**, which is a repair rather than a
/// restatement: the caller used to decide that a build it could not name was
/// not a swap at all, so it skipped the one half and took the other, and the
/// pair came apart in exactly the way this paragraph says it cannot. See
/// [`Running::landed`].
struct Running {
    playing: Vec<Option<Nodes>>,
    /// What a rollback restores, and the only way to name it: a rollback brings
    /// back a Set the stream never named again.
    previous: Vec<Option<Nodes>>,
}

impl Running {
    /// **Seed every slot from the material the run compiled**, addressed by the
    /// bytes that compile read.
    ///
    /// **At launch and for every windowed run**, not on `editable()`. A run with
    /// neither `--watch` nor `--mcp` is exactly the run where nothing will ever
    /// pick an edit up, so it is the run whose disk is most free to drift away
    /// from its picture — gating on `editable()` would leave that hole open.
    ///
    /// **No store, no disk, and nothing that can fail.** This used to open the
    /// store and write one artifact per node, which cost a plain windowed run a
    /// `.karakuri` directory it had never asked for — `docs/manual.md` says such
    /// a run "copies nothing and creates no directory", and it did until this
    /// function existed. The reason given for writing at launch was that a
    /// rollback onto the launch version emits `procedure` records naming these
    /// hashes and a replay must resolve them; that is true, and it is true only
    /// of a run with a recorder. So the bytes go in where a recorder is opened
    /// (see `App::resumed`) and where a save actually happens (see
    /// [`Sources::into_nodes`]), and a run that does neither writes nothing.
    ///
    /// It also used to *re-read* each `.kir` here, seconds after the compile
    /// that produced the deck. [`Placed::source`] is why it no longer can.
    fn at_launch(placed: &[Vec<Placed>], slots: usize) -> Running {
        let mut playing: Vec<Option<Nodes>> = vec![None; slots];
        for (slot, nodes) in placed.iter().enumerate().take(slots) {
            if nodes.is_empty() {
                continue;
            }
            playing[slot] = Some(stored_nodes(nodes.iter().map(Placed::node).collect()));
        }
        Running {
            previous: vec![None; playing.len()],
            playing,
        }
    }

    /// What `slot` is running, or `None` for a slot whose sources are not in the
    /// store.
    fn playing(&self, slot: usize) -> Option<&Nodes> {
        self.playing.get(slot).and_then(Option::as_ref)
    }

    /// **A build landed.** What the slot is now running, for the stream to say.
    ///
    /// `nodes` is `None` when that build's sources never reached the store —
    /// the watcher says so at the time, and the addresses it would have named
    /// do not exist. **That is still a swap**, and taking it as one is the
    /// whole of what this argument is for: the slot is on something new, so the
    /// version it was on becomes what a rollback restores, and the slot itself
    /// has no address until the next build lands. `None` comes back and no
    /// `procedure` record is written, because there is nothing to name.
    ///
    /// **The decision used to live in the caller**, which returned early on a
    /// build it could not name and so applied half a transition rule: the swap
    /// was skipped here and the matching rollback was not, and after one such
    /// pair the slot was recorded as running the version *before* the one on
    /// screen. `k` then wrote that version down and a recorded run put
    /// `procedure` records naming it into the stream — the picture and the file
    /// disagreeing, silently, which is the failure this whole type exists to
    /// make impossible. The first build of a run made it worse: `previous` was
    /// still `None`, so the rollback left the slot with no address at all and
    /// every later save refused on the grounds that the launch sources were not
    /// in the store, which was false.
    fn landed(&mut self, slot: usize, nodes: Option<Nodes>) -> Option<Nodes> {
        self.previous[slot] = self.playing[slot].take();
        self.playing[slot] = nodes;
        self.playing[slot].clone()
    }

    /// **A build was rolled back**, so the slot is running what it was running
    /// before it. What that is, for the stream to say.
    ///
    /// `None` where the version being restored has no address: a slot with
    /// nothing behind it at launch, or one whose `previous` is itself a build
    /// that never reached the store. With the launch version in `previous`, the
    /// first rollback of a slot restores it like any other, which is the case
    /// that used to fall through to nothing.
    fn rolled_back(&mut self, slot: usize) -> Option<Nodes> {
        let restored = self.previous[slot].take();
        self.playing[slot] = restored.clone();
        restored
    }
}

/// [`setfile::Node`]s as the addresses [`Nodes`] holds — the layer spelled the
/// way a record spells it, so a slot seeded at launch and a slot the watcher
/// rebuilt are the same shape.
fn stored_nodes(nodes: Vec<setfile::Node>) -> Nodes {
    nodes
        .into_iter()
        .map(|node| (setfile::kind_name(node.layer), node.index, node.hash))
        .collect()
}

/// **What a Set file says about the Set that is playing**, read off that Set.
///
/// Everything except the nodes, which are the one part a store has to be
/// involved in — see [`Save`].
///
/// **Seven of the eight are read from the Set and not from `Args`**, and the
/// eighth is the exception that has to earn itself — which is the decision this
/// function exists to hold. The reason is [`saving_capacities`]'s,
/// stated once and true of all of them: a writer with its own copy of the rule
/// records numbers the run was not using, and the file then describes a picture
/// nobody has seen. A run that has been *played* makes that concrete rather than
/// theoretical — a param moves through a record, and a slot rebuilt from an
/// edited `.kir` can change its own declared capacity underneath the flag that
/// was never given.
///
/// - **capacities** come per geometry from [`Set::source_capacities`] and not
///   from `Set::capacity`, which is the sum. One number for a two-geometry Set
///   is neither geometry's, and the writer refuses it.
/// - **params** are written *addressed*, every declaration of every node,
///   where `--save-set` writes only the `--param`s it was given. That is more
///   lines and it is the right ones: the Set holds a value per node whether an
///   operator wrote it or a `.kir` declared it, and a file that recorded only
///   the overrides would come back different the day the declaration changed.
/// - **bindings** come back with the ranges and curves they are riding at.
/// - **edges** are the run's — see `Live::edges` for why this one is not the
///   Set's, and why that is a copy of a value rather than of a rule.
/// - **camera** is the built-in orbit's six numbers, which a `camera` record
///   and a Set file both set from outside.
/// - **layering** is [`Set::layering`] and emphatically *not* `--merge`. This
///   is the surface `k` and the MCP tool reach, and both exist to write **what
///   is on screen**: the slot may have been filled by `--load-set` from a file
///   that recorded a `merge` the flags never mentioned, and it may have been
///   hot-swapped since. Asking the flag would write a file describing a Set
///   nobody was watching, which is [`saving_capacities`]' failure exactly.
/// - **live** is read off [`Set::inputs`] by [`selected_renderer`], for the
///   same reason and a louder one: nothing but the run can know it. There is no
///   flag that selects a renderer — `r` does, mid-performance — so the Set is
///   not merely the better source here, it is the only one.
/// - **seeds** are [`Set::source_salts`], one per geometry: what it *is* salted
///   with rather than what a position in `--set` would derive.
fn playing_values(
    set: &karakuri_engine::Set,
    edges: &[karakuri_engine::set::Edge],
) -> setfile::Owned {
    setfile::Owned {
        // Filled where a store is open, and nowhere else.
        nodes: Vec::new(),
        capacities: set.source_capacities(),
        params: set
            .params()
            .map(|(layer, index, key, value)| {
                karakuri_engine::ParamWrite::at(layer, index, key, value)
            })
            .collect(),
        bindings: set.bindings().to_vec(),
        edges: edges.to_vec(),
        camera: set.camera,
        layering: set.layering(),
        live: selected_renderer(set.inputs()),
        seeds: set.source_salts().to_vec(),
    }
}

/// **Which renderer a Set is folded to**, as a `merge` record spells it:
/// `Some(i)` where exactly one input is live, and `None` where every one of
/// them is.
///
/// **Every-live is checked first, and that decides the one-renderer case.** A
/// composited Set holding a single renderer has one live input, which is both
/// "all of them" and "exactly one" — and it is the first, because such a Set is
/// one nobody has selected in. Writing `live 0` for it would record a choice
/// that was never made, and `Record::Merge` is explicit that absent means every
/// input live rather than node 0.
///
/// **Anything else is `None` too, and the anything else has no producer.**
/// `mix::select` is the only thing that clears a `live` flag and it always
/// leaves exactly one set, so a fold with two of five live cannot be reached
/// from any surface this program has. If one ever is, `None` records the Set as
/// unselected — which is a fold the reader can build — rather than naming one
/// of them and calling that the choice.
fn selected_renderer(inputs: &[karakuri_engine::mix::Input]) -> Option<u32> {
    if inputs.iter().all(|input| input.live) {
        return None;
    }
    let mut live = inputs.iter().enumerate().filter(|(_, input)| input.live);
    match (live.next(), live.next()) {
        (Some((at, _)), None) => Some(at as u32),
        _ => None,
    }
}

/// One slot's nodes as the records they will be written as, with every source
/// put in the store first — which is what makes the hashes the file references
/// resolve on the way back in.
fn saving_nodes(
    store: &karakuri_store::store::Store,
    placed: &[Placed],
) -> Result<Vec<setfile::Node>, String> {
    placed
        .iter()
        .map(|node| node.put(store).map(|_| node.node()))
        .collect()
}

/// What a saved Set says each of its geometries runs at: **what the run was
/// actually drawing**.
///
/// This wrote `args.capacity` — the flag's number, or its default when no flag
/// was given — where the run itself asks [`capacity_for`], which prefers the
/// procedure's own declared default. So saving `lattice_shell` recorded 262144
/// and the run that saved it drew 32768, and loading the file back gave a
/// visibly different picture: a larger, smeared lattice.
///
/// **That falsified the one promise the format makes** — a run driven by the
/// file renders the same frame as the run whose flags wrote it
/// (`docs/adr/0066-a-flag-becomes-a-record-writer.md`). It was invisible while a Set was a pair, because the number was wrong in
/// the file and wrong again on the way back in; `--load-set` learning to honour
/// a recorded capacity is what made the two disagree out loud.
///
/// It changes the bytes of Set files saved by older builds of this program.
/// Those files still load — a `capacity` record has always meant what it says —
/// and they go on describing whatever they described. What changes is that new
/// ones describe the run.
fn saving_capacities(args: &Args, l1s: &[karakuri_ir::typed::Checked]) -> Vec<u32> {
    l1s.iter().map(|l1| capacity_for(args, l1)).collect()
}

/// What a saved Set says each of its geometries is salted with: **what the run
/// it describes will be salted with**, one number per geometry.
///
/// The same function the run itself asks, for the reason [`saving_capacities`]
/// exists — a writer with its own copy of the rule records numbers the run was
/// not using, and the file then describes a picture nobody has seen. Slot 0's,
/// because a Set file describes one Set and slot 0 is the one that gets saved.
///
/// **Derived today and recorded from here on.** Nothing on the command line
/// assigns a salt, so these are the ordinals — and writing them down is exactly
/// what stops them being ordinals: `docs/ir-spec.md` says *where it came from
/// stops mattering once it is recorded*, and from this line onward the file is
/// where the value lives. Reordering the paths in `--set` moves the colours of
/// a Set that was never saved and no longer moves the colours of one that was.
fn saving_seeds(args: &Args, l1s: &[karakuri_ir::typed::Checked]) -> Vec<u32> {
    salts_for(seed_for(0), recorded_salts(args, 0), l1s.len())
}

fn save_set(args: &Args, placed: &[Vec<Placed>], l1s: &[karakuri_ir::typed::Checked], id: &str) {
    let store = open_store(args);
    let Some(nodes) = placed.first().filter(|nodes| !nodes.is_empty()) else {
        eprintln!("karakuri-cli: --save-set needs a `.kir` chain to save");
        std::process::exit(1);
    };
    if placed.len() > 1 {
        eprintln!(
            "  only slot 0 is saved: a Set file describes one Set, and which Sets a deck \
             is holding belongs to a session"
        );
    }
    let camera = karakuri_engine::camera::Orbit::default();
    // **Every node, on the layer its own `kind` put it on**, and its source in
    // the store before the file that references it. The sorter already answered
    // the layer question for the engine — see [`sort_slot`] — so an L2, an L3
    // or a field is saved as what it is rather than refused for want of a slot
    // to write it in; the `put` is here rather than in the writer because this
    // is the caller holding paths that are still true. See `setfile::Node`.
    let nodes = match saving_nodes(&store, nodes) {
        Ok(nodes) => nodes,
        Err(e) => {
            eprintln!("karakuri-cli: {e}");
            std::process::exit(1);
        }
    };
    match setfile::save(
        &store,
        // `--save-set ID`, which is a flag an operator typed.
        Asked::Operator,
        id,
        setfile::Saving {
            nodes: &nodes,
            capacities: &saving_capacities(args, l1s),
            params: &args.overrides,
            bindings: &args.bindings,
            // **Which node fills each declared slot**, saved so the file
            // rebuilds: a slot nothing binds is refused where the Set is built,
            // so a Set file that dropped its edges would be one that no longer
            // loads.
            edges: &args.edges,
            camera: &camera,
            // **The flag's, because this path has no Set to ask.**
            // `--save-set` writes the material and exits before anything is
            // built, so what the run *would* play is what the flags say — and
            // for a one-shot they cannot be stale, since nothing has happened
            // to make them so. `k` and the MCP tool are the paths where that
            // stops being true, and `playing_values` reads the Set there.
            //
            // Through `layering_for`, so the flag and a `--load-set` file
            // cannot mean different things by it here than they mean anywhere
            // else — `--load-set` beside `--save-set` is refused, so the file
            // half is only ever the default today, and one derivation is still
            // one derivation.
            layering: layering_for(args, 0, recorded_layering(args, 0)),
            // **No selection, because nothing has selected.** `r` is a key
            // pressed at a running frame and there is no flag for it, so a
            // one-shot save has none to record — see `recorded_live`.
            live: None,
            seeds: &saving_seeds(args, l1s),
        },
    ) {
        Ok(()) => eprintln!(
            "wrote set `{id}` to {} — load it with `--load-set {id}`",
            args.store.display()
        ),
        Err(e) => {
            eprintln!("karakuri-cli: {e}");
            std::process::exit(1);
        }
    }
}

/// Read a Set file and fold what it says back into the arguments, so everything
/// downstream is driven the way the flags drive it.
///
/// **Every note is printed.** A Set file this build cannot honour in full still
/// loads, and the alternative — succeeding quietly — is the material being
/// subtly not what was saved with nothing anywhere saying so.
fn load_set(args: &mut Args, id: &str) -> setfile::Loaded {
    let store = open_store(args);
    let loaded = match setfile::load(&store, id) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("karakuri-cli: {e}");
            std::process::exit(1);
        }
    };
    for note in &loaded.notes {
        eprintln!("  {note}");
    }
    // **`capacity_given` too, or the number is read and then discarded.** It
    // is what makes `capacity_for` stop falling back to the procedure's own
    // declared default — and a Set file that recorded a capacity is somebody
    // having said so as much as `--capacity` is. Without it, `--capacity 100000
    // --save-set x` followed by `--load-set x` ran at whatever the `.kir`
    // declared, which falsifies the one promise a Set file makes.
    // The first geometry's, because that is what one number can hold; the rest
    // travel per geometry on `from_set` and are applied where the sources are
    // in hand. This one is still needed as the flag: it is what a rebuilt slot
    // is given — see `Watch::new` — and what the status line prints.
    if let Some(capacity) = loaded.capacities.first().copied().flatten() {
        args.capacity = capacity;
        args.capacity_given = true;
    }
    // Appended rather than replacing: a `--param` or `--bind` given alongside
    // `--load-set` is the operator overriding the file, and the later value is
    // what `build` applies.
    let mut overrides = loaded.params.clone();
    overrides.append(&mut args.overrides);
    args.overrides = overrides;
    let mut bindings = loaded.bindings.clone();
    bindings.append(&mut args.bindings);
    args.bindings = bindings;
    // **The file's edges, then the flags'**, on the terms the params above
    // follow: an `--edge` given beside `--load-set` is the operator rebinding a
    // slot the file bound. It *replaces* rather than piling up, which is where
    // this differs from a `--param` — two edges on one slot are refused where
    // the Set is built, so appending both would turn an override into a
    // refusal. Only the slot the flag names is dropped; the file's other edges
    // stand.
    let mut edges = loaded.edges.clone();
    edges.retain(|e| {
        !args
            .edges
            .iter()
            .any(|given| given.node == e.node && given.slot == e.slot)
    });
    edges.append(&mut args.edges);
    args.edges = edges;
    args.from_set = Some(FromSet {
        salts: loaded.salts.clone(),
        camera: loaded.camera,
        // **Carried rather than folded into `--merge`**, and read back through
        // [`layering_for`] — which is where the flag and the file meet, once,
        // for everything that builds this slot or writes it out again.
        layering: loaded.layering,
        live: loaded.live,
        capacities: loaded.capacities.clone(),
    });
    loaded
}

/// **Whether anything in this run can write a `.kir`.** `--watch` and `--mcp`
/// are the two things that edit a procedure; a render or a replay opens every
/// file read-only.
///
/// A function rather than a `let`, because three things turn on it and they
/// have to turn on the same one: what gets copied into the scratch, what gets
/// compiled out of `args.sets` rather than out of the Set file directly, and —
/// since a rebuilt slot's sources are what a live save writes — whether the
/// watchers put what they build into the store. The third arrived on the far
/// side of `main`, in `App::resumed`, which is what made the copy worth
/// removing.
fn editable(args: &Args) -> bool {
    args.watch || args.mcp.is_some()
}

fn main() {
    let args = parse_args();

    // **A Set file replaces the paths and the flags together**, because it
    // carries both: the material and everything the flags were standing in for
    // until it existed. Handled before anything is compiled, so a run either
    // takes its material from a file or from the command line and never from
    // half of each.
    let mut args = args;
    let loaded = args.load_set.clone().map(|id| load_set(&mut args, &id));

    // **Before anything reads a `.kir`**, so that the deck, the watcher and the
    // MCP surface all get the same rewritten paths from one place — they each
    // read `args.sets` and none of them has to know this happened.
    //
    // Only when the run can write one. `--watch` and `--mcp` are the two things
    // that edit a procedure; a render or a replay opens every file read-only,
    // so copying would leave a directory behind for a run that is supposed to
    // be a function of its arguments. See `karakuri-environment`'s `scratch.rs`
    // for the rest of the reasoning, including what happened when there was no
    // such place.
    // The run's edit history, seeded below from the scratch. `None` for a run
    // that cannot be edited, which is the same condition the scratch has and
    // for the same reason: nothing writes a `.kir`, so there is no version to
    // preserve and no directory to leave behind.
    let mut snapshots: Option<history::Shared> = None;
    let editable = editable(&args);
    if editable {
        let root = args.store.clone();
        // A loaded Set names its procedures by hash and has no file anywhere,
        // so it is written into the scratch first and then joins `args.sets` as
        // ordinary material. Everything downstream — the compile, the watcher,
        // the MCP surface — then treats it exactly like a `--set` chain, which
        // is what makes a saved Set editable rather than only playable.
        if let Some(loaded) = &loaded {
            // Every node the file named, in node order — which starts with the
            // geometries, so the first path is the L1 the pair below wants and
            // the rest are sorted by their own `kind` like any `--set` list.
            // **Slot 0's own names**, because the pair below is inserted at
            // the head of `args.sets` and a scratch file carries the slot it
            // belongs to — the same `A0-drift.kir` `materialise` writes a few
            // lines down and `crates/karakuri`'s `loading` writes on a library
            // load. One rule for one directory: `scratch::node_name`.
            let written: Result<Vec<PathBuf>, String> = loaded
                .nodes()
                .enumerate()
                .map(|(at, (checked, src))| {
                    scratch::place(&root, &scratch::node_name(0, at, &checked.name), src)
                })
                .collect();
            match written {
                // **Each path keeps the name the file gave that node**, so a
                // loaded Set that is then edited resolves its edges against the
                // spellings it was saved with rather than against the procedure
                // names the scratch happens to file it under.
                Ok(paths) => {
                    let mut named = paths
                        .iter()
                        .cloned()
                        .zip(loaded.node_names())
                        .map(|(path, name)| Named { name, path });
                    let head = named.next().expect("a Set file names at least an L1");
                    args.sets.insert(0, (head, named.collect()))
                }
                Err(e) => {
                    eprintln!("karakuri-cli: {e}");
                    std::process::exit(1);
                }
            }
        }
        // **Slot by slot rather than one flat list**, because the name of a
        // copy carries the slot it belongs to: the same file given to two
        // slots becomes two files, so an edit moves the slot whose file it is
        // and no other. See `scratch.rs`'s header for the rule this replaced.
        match scratch::materialise(
            &root,
            args.sets.iter_mut().map(|(l1, l4s)| {
                std::iter::once(&mut l1.path).chain(l4s.iter_mut().map(|n| &mut n.path))
            }),
        ) {
            Ok(dir) => {
                eprintln!(
                    "scratch: {} — every slot runs from its own copy here, so the files you \
                     named are not written to. Point an editor at these",
                    dir.display()
                );
                // **What each slot is actually watching, named.** One preset
                // given to four slots is four files now, which is the point
                // and is also the thing an operator cannot guess: the line
                // above says where, and this says which is whose.
                for (slot, (l1, l4s)) in args.sets.iter().enumerate() {
                    let names: Vec<String> = std::iter::once(&l1.path)
                        .chain(l4s.iter().map(|n| &n.path))
                        .map(|p| {
                            p.file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| p.display().to_string())
                        })
                        .collect();
                    eprintln!("  slot {slot}: {}", names.join(" + "));
                }
            }
            Err(e) => {
                eprintln!("karakuri-cli: {e}");
                std::process::exit(1);
            }
        }
        // **Seeded from the scratch, before the first compile.** The first edit
        // records what replaced the original; without this, what it replaced was
        // never written down and the first edit is the one that cannot be undone.
        let shared = history::Snapshots::shared(&args.store);
        history::seed(
            &shared,
            args.sets.iter().enumerate().map(|(slot, (l1, l4s))| {
                (
                    slot,
                    std::iter::once(l1.path.as_path())
                        .chain(l4s.iter().map(|n| n.path.as_path()))
                        .collect(),
                )
            }),
        );
        snapshots = Some(shared);
    }

    eprintln!("compiling:");
    let mut procs: Vec<Material> = Vec::new();
    // Where each slot's files ended up, slot for slot beside `procs`. **Kept
    // parallel**, empty entry and all, because everything downstream indexes it
    // by slot: a list that skipped the slot a Set file filled would hand slot
    // 0's saver slot 1's files. See [`Placed`].
    let mut placed: Vec<Vec<Placed>> = Vec::new();
    // Only when it was *not* materialised into the scratch above. An editable
    // run compiles it out of `args.sets` with everything else, which is the
    // point: one path, so a loaded Set can be watched and rewritten.
    if let Some(loaded) = loaded.filter(|_| !editable) {
        eprintln!("  slot 0: set `{}`", loaded.id);
        // **The names the file recorded**, which are what its edges are
        // written against — a load that dropped them would be a load whose
        // slots resolve to nothing.
        let names = loaded.names.clone();
        procs.push(Material {
            l1s: loaded.l1s,
            l2s: loaded.l2s,
            l3s: loaded.l3s,
            fields: loaded.fields,
            l4s: loaded.l4s,
            names,
        });
        // No files behind it — the sources came out of the store by hash. The
        // empty entry is what keeps `placed` indexed by slot; nothing saves
        // this slot, since `--load-set` with `--save-set` is refused and a
        // session's head is the file itself.
        placed.push(Vec::new());
    }
    for (slot, (l1, rest)) in args.sets.iter().enumerate() {
        let (material, nodes) = sort_slot(slot, l1, rest);
        procs.push(material);
        placed.push(nodes);
    }

    // Saving is a one-shot: it writes what the flags say and stops, on the same
    // terms as `--render`. Running afterwards would leave an operator unsure
    // whether what they are watching is what was written.
    if let Some(id) = &args.save_set {
        let geometries = procs.first().map(|m| m.l1s.as_slice()).unwrap_or(&[]);
        save_set(&args, &placed, geometries, id);
        return;
    }

    if let Some(id) = args.replay.clone() {
        replay_session(&args, &id);
        return;
    }

    match args.render_to.clone().or(args.seq_to.clone()) {
        Some(path) => {
            let gpu = Gpu::headless().expect("no GPU");
            let (w, h) = args.canvas;
            check_canvas(&gpu.device, w, h);
            // Fixed, never watching: an offscreen run is a function of its
            // inputs, and a save landing halfway through a sequence would make
            // it a function of the operator's editor as well.
            // **Nothing to re-aim, and nothing that could ask.** This path
            // watches nothing and serves no MCP, so every entry would be
            // `None` and no `wire_input` can reach it.
            let (mut deck, _) = build_deck(&gpu, &procs, &args, false, false, w, h, None, None);
            eprintln!(
                "rendering the mix of {} Set{}, {w}x{h}, {} frames, \
                 {} at exposure {:.2} -> {}",
                deck.slot_count(),
                if deck.slot_count() == 1 { "" } else { "s" },
                args.frames,
                op_name(args.look.op),
                args.look.exposure,
                path.display()
            );
            let result = if args.seq_to.is_some() {
                render::to_sequence(&gpu, &mut deck, args.look, w, h, args.frames, &path)
            } else {
                render::to_png(&gpu, &mut deck, args.look, w, h, args.frames, &path)
            };
            if let Err(e) = result {
                eprintln!("{e}");
                std::process::exit(1);
            }
            report_live_counts(&gpu, &deck);
        }
        None => {
            let event_loop = EventLoop::new().expect("event loop");
            event_loop.set_control_flow(ControlFlow::Poll);
            event_loop
                .run_app(&mut App {
                    args,
                    procs: Some(procs),
                    placed,
                    live: None,
                    snapshots,
                })
                .expect("run");
        }
    }
}

/// The element live count of every slot. **A stall per slot** — it copies four
/// bytes off the GPU and blocks until the queue drains — which is why this is
/// called once, after the last frame, and never from the status line. See
/// `Set::live_count`, which says so at the definition.
fn report_live_counts(gpu: &Gpu, deck: &Deck) {
    eprintln!("elements at the end of the run (a stall; not printed while running):");
    for slot in 0..deck.slot_count() {
        let set = deck.slot(slot).set();
        eprintln!(
            "  slot {slot}: {} live of {} allocated",
            set.live_count(&gpu.device, &gpu.queue),
            set.capacity()
        );
    }
}

/// What a loaded Set file said its geometries run at, for the slot it filled.
///
/// **Slot 0 and nothing else**, on the same terms as its seed and its camera:
/// `--load-set` fills that slot and every other slot comes from `--set`.
fn recorded_capacities(args: &Args, slot: usize) -> &[Option<u32>] {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => &from_set.capacities,
        _ => &[],
    }
}

/// What a loaded Set file said its geometries are salted with, for the slot it
/// filled. **Slot 0 and nothing else**, on the same terms as its capacities.
fn recorded_salts(args: &Args, slot: usize) -> &[Option<u32>] {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => &from_set.salts,
        _ => &[],
    }
}

/// Where a loaded Set file aimed the built-in camera, for the slot it filled.
/// **Slot 0 and nothing else**, on the same terms as its capacities and its
/// salts — and `None` for a file that recorded no `camera` record, or one whose
/// record named a camera that is a procedure and was reported and dropped on
/// the way in.
///
/// **One reading, used twice.** The Set built at startup takes it and so does
/// the watcher that restates it on every rebuild; working it out in two places
/// is how a rebuild came to aim somewhere the startup did not.
fn recorded_camera(args: &Args, slot: usize) -> Option<karakuri_engine::camera::Orbit> {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => from_set.camera,
        _ => None,
    }
}

/// **Whether a slot composites its renderers or overdraws them** — the one
/// reading, from the flag and the file together.
///
/// **Either saying so is enough, and that is a decision rather than a
/// coincidence.** `--merge N` can only turn compositing *on*: there is no
/// spelling that turns it off, because the record's absence is what overdraw is
/// and a flag that could say `false` would be a second spelling of not typing
/// it. So `--load-set X --merge 0` on a file that already records a `merge` is
/// two ways of asking for the same thing, and a file that records none plus
/// `--merge 0` is the flag adding what the file did not say. The one case that
/// could have been a contest — a composited file with `--merge` *left off* —
/// is not one: leaving a flag off is not a statement, and treating it as one
/// would make a loaded preset silently overdraw exactly as it did before this
/// record existed.
///
/// **One reading, used everywhere**, on [`recorded_camera`]'s terms: the Set
/// built at startup takes it, the watcher that restates it on every rebuild
/// takes it, and the writer that saves the slot back out takes it. Working it
/// out in two places is how a rebuild came to aim a camera where the startup
/// did not.
fn layering_for(
    args: &Args,
    slot: usize,
    recorded: karakuri_engine::set::Layering,
) -> karakuri_engine::set::Layering {
    if recorded == karakuri_engine::set::Layering::Composite || args.merge.contains(&slot) {
        karakuri_engine::set::Layering::Composite
    } else {
        karakuri_engine::set::Layering::Overdraw
    }
}

/// **What a loaded Set file said about its layering**, for the slot it filled —
/// [`recorded_camera`]'s shape and its rule: slot 0 and nothing else, since
/// `--load-set` fills that slot and every other comes from `--set`.
///
/// Separate from [`layering_for`] because the replay path has a recorded
/// layering in hand without ever touching `args.from_set` — its Set file is the
/// head of a session stream — and both readings have to meet the flag through
/// the same function. That is [`recorded_capacities`] and [`capacities_for`]'s
/// split, for its reason.
fn recorded_layering(args: &Args, slot: usize) -> karakuri_engine::set::Layering {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => from_set.layering,
        _ => karakuri_engine::set::Layering::Overdraw,
    }
}

/// Which renderer a loaded Set file left folded to, for the slot it filled.
/// **Slot 0 and nothing else**, on the same terms as its camera and its
/// capacities — and `None` for a file that recorded no selection, which is
/// every input live and is the state a Set nobody selected in comes up in.
///
/// **No flag stands beside this one.** There is no `--select`: a selection is
/// something an operator makes with `r` while watching, so the only thing that
/// can put one in before the first frame is a file that recorded one.
fn recorded_live(args: &Args, slot: usize) -> Option<u32> {
    match (slot, args.from_set.as_ref()) {
        (0, Some(from_set)) => from_set.live,
        _ => None,
    }
}

/// **One slot's watcher, and the aim it is pointed at.**
///
/// **The run holds this so that a rewiring can reach the build worker.** The
/// wiring a slot rebuilds with is not on disk anywhere — `Args::edges` at
/// launch, `watch::Watch::edges` on every rebuild, [`Live::edges`] at a save —
/// so an edge written during a show has to be *handed* to the watcher, and
/// `watch::Watch::aimed_by` is the one way in. It is deliberately not an
/// install: `Deck::install` would put a Set on air that nothing measured, and
/// an aim instead says *look at this instead* and lets go, after which
/// everything is the path an edit already takes — compiled on the worker,
/// swapped at a frame boundary, judged against the budget and rolled back on
/// its own if it costs too much. That is [`mcp::WireRequest`]'s second point,
/// and reaching it this way is why there is no second route into a slot.
///
/// **The aim is kept and not only the sender**, because an `Aim` is every field
/// of the slot's identity and *anything left out comes back as the outgoing
/// slot's* — a fold silently un-selected, a camera back at `Orbit::default()`,
/// salts that repaint every element. A rewiring changes one field of thirteen,
/// so the other twelve have to be restated from somewhere, and this is that
/// somewhere: what the watcher was started at, moved forward by every aim sent
/// since.
struct Aiming {
    /// The other end of `watch::Watch::aimed_by`'s channel, for this slot's
    /// watcher and no other. A watcher re-pointed through somebody else's
    /// sender would rebuild a deck nobody named.
    aim: std::sync::mpsc::Sender<watch::Aim>,
    /// **Where that watcher is pointed**, kept in step with what has been sent:
    /// the values it was constructed with until the first aim, and the last aim
    /// after that. A copy that stopped being updated would restate a stale
    /// wiring on the *second* rewiring of a run, which is the hardest version
    /// of this mistake to see.
    at: watch::Aim,
}

impl Aiming {
    /// **Point the watcher at the same material with `edges` instead**, and
    /// answer whether it is still there to be pointed.
    ///
    /// `Err` is a build worker that has ended — the receiver is gone — which is
    /// a run shutting down. It is reported rather than swallowed: the edge is
    /// in the run's wiring either way, and *nothing will rebuild* is a
    /// different fact from *the slot is recompiling*.
    fn re_aim(&mut self, edges: Vec<karakuri_engine::set::Edge>) -> Result<(), ()> {
        self.at.edges = edges;
        self.aim.send(restated(&self.at)).map_err(|_| ())
    }
}

/// One aim, said again — because `watch::Aim` is not `Clone` and a re-point
/// restates every field of it.
///
/// **No `..` on either side of this**, which is `Watch::repointed`'s own rule
/// met from the sending end: it destructures with no `..` so that a field
/// added to `Aim` cannot be left behind, and a *sender* that filled the new
/// field with a default would defeat that from here. The compiler names all
/// thirteen, so the day a fourteenth arrives this stops compiling rather than
/// quietly re-aiming a slot at it.
fn restated(aim: &watch::Aim) -> watch::Aim {
    let watch::Aim {
        head,
        rest,
        layering,
        live,
        capacity,
        seed_salt,
        salts,
        camera,
        overrides,
        published,
        bindings,
        edges,
        authorities,
    } = aim;
    watch::Aim {
        head: head.clone(),
        rest: rest.clone(),
        layering: *layering,
        live: *live,
        capacity: *capacity,
        seed_salt: *seed_salt,
        salts: salts.clone(),
        camera: *camera,
        overrides: overrides.clone(),
        published: published.clone(),
        bindings: bindings.clone(),
        edges: edges.clone(),
        authorities: authorities.clone(),
    }
}

/// **Every edge a client asked for on one frame, applied to the run's wiring
/// and answered.**
///
/// This is [`mcp::WireRequest`]'s three points, and it is a free function so
/// that all three are checkable without a window, a GPU or a `Deck` — the
/// wiring, the re-aim and the sentence are the whole of what this decides, and
/// none of them needs one.
///
/// # Replace, keyed on the input
///
/// An edge is dropped and the new one appended, keyed on `(node, slot)` — the
/// node that declares the input and what its procedure calls it — which is
/// `--edge`'s own spelling beside `--load-set` a few hundred lines up, down to
/// the `retain` and the order it leaves behind. It is forced rather than
/// chosen: `SetError::SlotBoundTwice` refuses two edges on one input where the
/// Set is built, so an append would make the *second* call on an input a
/// refusal and leave a model unable to change its mind.
///
/// **The key does not include the deck slot, because the run's wiring does
/// not.** `Live::edges` is one list for the whole run and an edge naming a node
/// a Set has not got is passed over where the Set is built — see
/// `karakuri_engine::set::Wiring::edges`. So a request names a deck slot to say
/// *which slot rebuilds*, and two slots holding a node of the same name share
/// one entry in this list, exactly as they do when `--edge` is typed on the
/// command line.
///
/// # A slot this deck does not hold
///
/// **Refused, in [`no_such_slot`]'s words, and nothing is rewired** — the
/// decision [`Live::save_set`] already makes for a save and for its reason: a
/// key press cannot name a slot the deck has not got and a tool call can, and
/// this is the guard that does not depend on the surface that asked having one.
/// The MCP server checks the number against `mcp::Slots` before it sends, so a
/// model meets its refusal there; the deck's own count is a thing only the run
/// knows, and this is where it is known.
///
/// **Residency is deliberately not consulted.** An off-air slot is wired and
/// rebuilt like any other: a slot is prepared while it is dark and put on air
/// afterwards, so refusing an edge on an allocated slot would forbid the one
/// order an operator actually works in. What a rebuild of a dark slot costs is
/// the same as any other rebuild and is judged the same way.
///
/// # The same input wired twice on one frame
///
/// **Every request is applied, in the order it arrived, and the last one is
/// what the run is wired with** — a rewiring is a model changing its mind, and
/// the frame a change of mind lands on is not something a client controls. One
/// aim per slot goes out after all of them are in the list, so the rebuild
/// carries the settled wiring rather than an intermediate one, and
/// `Watch::repointed` takes only the newest aim anyway.
///
/// **A request the same frame overwrote is told so**, which is the only part of
/// this that costs anything: its edge *was* written and then replaced, and a
/// reply saying only "wired" would be a true sentence about a state the run no
/// longer holds by the end of the frame it was sent on.
///
/// # What each answer is
///
/// `Err` is the refusal above and nothing else. A slot with no watcher is not a
/// failure — the edge is in the run's wiring, a `save_set` records it, and the
/// only thing missing is the rebuild, which the sentence says. That matches the
/// note `mcp::wire_input` already adds for a run started without `--watch`.
fn rewired(
    asked: &[(usize, karakuri_engine::set::Edge)],
    edges: &mut Vec<karakuri_engine::set::Edge>,
    aims: &mut [Option<Aiming>],
    slot_count: usize,
) -> Vec<Result<String, String>> {
    // **Every edge into the list before any watcher is re-aimed**, so that a
    // frame carrying two of them rebuilds once, at the wiring the frame ended
    // with.
    let mut said: Vec<Option<Result<String, String>>> = asked.iter().map(|_| None).collect();
    let mut named: Vec<usize> = Vec::new();
    for (at, (slot, edge)) in asked.iter().enumerate() {
        if !slot_in_range(*slot, slot_count) {
            said[at] = Some(Err(format!(
                "{}, and nothing was rewired",
                no_such_slot(*slot, slot_count)
            )));
            continue;
        }
        edges.retain(|held| !(held.node == edge.node && held.slot == edge.slot));
        edges.push(edge.clone());
        if !named.contains(slot) {
            named.push(*slot);
        }
    }
    // `None` for a slot with no watcher at all, `Some(false)` for one whose
    // build worker has ended: two different things to say, and neither of them
    // is "the slot is recompiling".
    let rebuilding: Vec<(usize, Option<bool>)> = named
        .into_iter()
        .map(|slot| {
            let state = aims
                .get_mut(slot)
                .and_then(Option::as_mut)
                .map(|aiming| aiming.re_aim(edges.clone()).is_ok());
            (slot, state)
        })
        .collect();
    for (at, (slot, edge)) in asked.iter().enumerate() {
        if said[at].is_some() {
            continue;
        }
        let mut line = format!(
            "slot {slot}: wired `{}.{}={}`",
            edge.node, edge.slot, edge.to
        );
        // Keyed on the input alone, like the replacement above: whichever deck
        // slot a later request named, it took this entry in the run's wiring.
        //
        // **Only ones that were applied**, which is the whole reason this is a
        // second pass rather than a lookahead in the first: a later request
        // refused for its slot number wrote nothing, and telling this one it
        // had been replaced by an edge that never landed would be the same lie
        // in the other direction.
        let over = asked[at + 1..].iter().find(|(later_slot, later)| {
            slot_in_range(*later_slot, slot_count)
                && later.node == edge.node
                && later.slot == edge.slot
        });
        if let Some((_, later)) = over {
            let _ = write!(
                line,
                ", and a later request on this frame replaced it with `{}` — the run is \
                 wired with that one and it is what the rebuild carries",
                later.to
            );
        }
        let state = rebuilding
            .iter()
            .find(|(named, _)| named == slot)
            .and_then(|(_, state)| *state);
        let tail = match state {
            Some(true) => {
                " — the slot is recompiling with it, and `swap_outcome` says what the build \
                 made of it"
            }
            Some(false) => {
                " — this slot's build worker has ended, so nothing will rebuild: the edge is \
                 the run's from here on and a `save_set` of this slot records it"
            }
            None => {
                " — this slot has no watcher, so nothing rebuilds: what is on air was built \
                 with the wiring the run started with, and a `save_set` of this slot records \
                 the edge"
            }
        };
        line.push_str(tail);
        said[at] = Some(Ok(line));
    }
    // Every entry was filled by one of the two loops above: the first answers
    // the refusals and the second answers everything it skipped.
    said.into_iter().map(Option::unwrap).collect()
}

/// `meters` is false for the offscreen paths: a `--render` has nobody to show
/// a level to, and a meter that nothing reads is a compute pass and a staging
/// ring per frame for no reason. That is the whole point of it being opt-in.
#[allow(clippy::too_many_arguments)]
fn build_deck(
    gpu: &Gpu,
    procs: &[Material],
    args: &Args,
    watch: bool,
    meters: bool,
    width: u32,
    height: u32,
    // Where a rebuilt procedure's sources are put and reported. `None` and no
    // watcher touches a store. See `watch::Watch::stored` for why this is no
    // longer the recorder's switch.
    stored: Option<(
        std::sync::Arc<karakuri_store::store::Store>,
        std::sync::mpsc::Sender<watch::Built>,
    )>,
    // The run's edit history, shared by every slot's watcher and already
    // holding what the run started with. `None` for a run that cannot be
    // edited — see `history`.
    snapshots: Option<history::Shared>,
    // **The deck, and where each of its slots can be re-pointed** — one entry
    // per slot, `None` for every slot with no watcher behind it. The senders
    // are made where the watchers are, because a sender paired with the wrong
    // slot's watcher would rebuild a deck the caller did not name; see
    // [`Aiming`].
) -> (Deck, Vec<Option<Aiming>>) {
    // One flag per binding, shared across every slot: a binding names a layer
    // and a param, and a deck of four slots is four chances for it to land.
    let mut attached = vec![false; args.bindings.len()];
    // **Resolved once per slot, and handed to everything that needs it.** A
    // slot's salts are what its Set is built with *and* what its watcher
    // restates on every rebuild; working them out in two places is how a save
    // under `--watch` would come back in different colours from the Set it
    // rebuilt.
    let salts: Vec<Vec<u32>> = procs
        .iter()
        .enumerate()
        .map(|(slot, material)| {
            salts_for(
                seed_for(slot),
                recorded_salts(args, slot),
                material.l1s.len(),
            )
        })
        .collect();
    let built: Vec<(HotSwap, Option<Aiming>)> = procs
        .iter()
        .enumerate()
        .map(|(slot, material)| {
            let (l1, l2s, l3s, fields, l4s) = (
                material.l1s.as_slice(),
                &material.l2s,
                material.l3s.as_slice(),
                material.fields.as_slice(),
                &material.l4s,
            );
            // **Read once for both the Set and its watcher** — see
            // [`recorded_camera`].
            let camera = recorded_camera(args, slot);
            // **The flag and the file, resolved once** — see [`layering_for`].
            // Read here rather than at each use for [`recorded_camera`]'s
            // reason and with the same symptom: the watcher restates a layering
            // on every rebuild, so a second derivation that disagreed would
            // turn the first save of any `.kir` in the slot into a Set that
            // stopped compositing, with nothing said.
            let layering = layering_for(args, slot, recorded_layering(args, slot));
            let live = recorded_live(args, slot);
            let set = build(
                gpu,
                l1,
                l2s,
                l3s,
                fields,
                l4s,
                layering,
                &material.names,
                &args.edges,
                &capacities_for(args, l1, recorded_capacities(args, slot)),
                &mut attached,
                &args.overrides,
                &args.bindings,
                &args.published,
                // **The Set's own seed is its first geometry's salt**, which
                // is what one number can hold and what a file recording one
                // `seed` has always meant by it. A Set file's own where it
                // recorded one, so a saved Set reproduces rather than being
                // re-salted by the slot it lands in — which only ever applies
                // to slot 0, since `--load-set` fills that slot and the rest
                // come from `--set`.
                salts[slot]
                    .first()
                    .copied()
                    .unwrap_or_else(|| seed_for(slot)),
                &salts[slot],
                camera,
                live,
            );
            if watch {
                // **What this slot's watcher is pointed at, stated once.** It
                // used to be thirteen arguments to `Watch::new` and it is now
                // an `Aim` those arguments are read out of, because a re-point
                // has to restate every one of them — see [`Aiming`] and
                // `watch::Aim`, whose own documentation is that *anything left
                // out comes back as the outgoing slot's*. Building the aim here
                // and starting the watcher from it is what makes "where this
                // watcher is pointed" one value rather than two lists that
                // agree today.
                let at = watch::Aim {
                    // **With the names, not only the paths.** A rebuild
                    // resolves its edges against them, and a watcher that
                    // handed the sort bare paths would rename every node
                    // on the first save.
                    head: args.sets[slot].0.clone(),
                    rest: args.sets[slot].1.clone(),
                    // **The same reading the Set was built with**, and
                    // the whole reason it is read once above: a rebuild
                    // restates the layering rather than re-deriving it, so
                    // a slot that loaded a composited Set file is still
                    // compositing after the first save of a `.kir` in it.
                    // A rebuild that let this be worked out again from
                    // `--merge` alone would quietly discard what was
                    // loaded — which is `Watch::camera`'s failure, in the
                    // one place the picture does not even come back.
                    layering,
                    // **And the selection with it**, for the same reason
                    // and in the same breath: a fold restated to every
                    // input live is a rebuild silently un-selecting what
                    // the file selected. See `Watch::live`.
                    live,
                    capacity: args.capacity_given.then_some(args.capacity),
                    seed_salt: salts[slot]
                        .first()
                        .copied()
                        .unwrap_or_else(|| seed_for(slot)),
                    // **Restated on every rebuild rather than derived
                    // there.** A slot filled from a Set file is running at
                    // the salts that file recorded, and a rebuild that
                    // derived its own would change every colour in it on the
                    // next save of a `.kir`.
                    salts: salts[slot].clone(),
                    // **The same reading the Set was built with**, and
                    // stated as an `Orbit` rather than as the `Option` it
                    // was read as: a slot that loaded no `camera` record is
                    // running at `Orbit::default()`, because that is what
                    // `Set::build_many` builds and what leaving it
                    // unassigned above therefore means. See `Watch::camera`
                    // for why the option buys nothing past this line.
                    camera: camera.unwrap_or_default(),
                    overrides: args.overrides.clone(),
                    published: args.published.clone(),
                    bindings: args.bindings.clone(),
                    // **The run's wiring, which is one list and not one per
                    // slot** — see `Live::edges`. `wire_input` replaces an
                    // entry in it and re-aims this watcher with the result.
                    edges: args.edges.clone(),
                    authorities: Vec::new(),
                };
                // **Opened for every watched slot rather than only under
                // `--mcp`**, because the sender is what pairs a slot with its
                // own watcher and pairing it later would mean holding the
                // thirteen values above somewhere else to do it. A run nobody
                // rewires never sends on it and it costs a `Sender`.
                let (aim, aimed) = std::sync::mpsc::channel();
                // One worker and one watcher per slot, over that slot's own
                // two files. That is what makes "the slot whose files changed"
                // the thing that rebuilds: no slot can see another's edit.
                let swap = HotSwap::new(&gpu.device, &gpu.queue, set, args.budget_ms, {
                    let watcher = watch::Watch::new(
                        slot,
                        at.head.clone(),
                        at.rest.clone(),
                        at.layering,
                        at.live,
                        at.capacity,
                        at.seed_salt,
                        at.salts.clone(),
                        at.camera,
                        at.overrides.clone(),
                        at.published.clone(),
                        at.bindings.clone(),
                        at.edges.clone(),
                        at.authorities.clone(),
                    )
                    // **Where a re-point arrives.** Every slot this program
                    // watches can be re-aimed from the render thread, which is
                    // how an edge written over MCP reaches the build worker:
                    // `Watch::aimed_by`'s own documentation says a load *"lets
                    // go"* and everything after it is the path an edit already
                    // takes, which is exactly what `WireRequest` asks for.
                    .aimed_by(aimed);
                    // The history is kept whether or not a session is
                    // being recorded: the two answer different questions —
                    // see `Watch::snapshots`. One `Shared` across every
                    // slot, because it is also what the launch-time seed
                    // wrote into.
                    let watcher = match &snapshots {
                        Some(shared) => watcher.snapshotting_to(shared.clone()),
                        None => watcher,
                    };
                    // **Whenever the run is editable**, which is where the
                    // caller decides it — the watcher only has to be told
                    // where. This used to be "only when a session is being
                    // recorded", and see `Watch::stored` for what that cost.
                    Box::new(match &stored {
                        Some((store, tx)) => watcher.storing_to(store.clone(), tx.clone()),
                        None => watcher,
                    })
                });
                (swap, Some(Aiming { aim, at }))
            } else {
                // **No watcher and therefore nothing to re-aim.** An edge
                // written into a run like this one is still the run's — a save
                // records it — and nothing rebuilds, which is what
                // [`rewired`] says in that slot's sentence.
                (HotSwap::fixed(set), None)
            }
        })
        .collect();
    let (swaps, aims): (Vec<HotSwap>, Vec<Option<Aiming>>) = built.into_iter().unzip();
    let mut deck = Deck::new(&gpu.device, swaps, width, height);
    // The session's one local oscillator, before the first frame. `SEED` is
    // the seed every noise stream comes off — the same explicit seed the Sets
    // are salted from, so a run is reproducible from its arguments alone.
    deck.set_signals(Signals::new(args.bpm, u64::from(SEED)));
    if meters {
        deck.enable_meters(&gpu.device);
    }
    // Once, not per slot: what attached, and how far each will actually move.
    // The confidence is the part worth printing — a binding to an invented
    // signal moving a tenth of the way is the system working, and an operator
    // who does not know that reads it as a broken binding.
    // **Only the ones that attached.** Describing a binding that landed
    // nowhere told an operator it "decides that param outright" one line after
    // saying it was ignored, which is the failure the confidence display had
    // and had fixed — reappearing one step further along.
    for (binding, _) in args.bindings.iter().zip(&attached).filter(|(_, on)| **on) {
        eprintln!("  {}", describe(binding, deck.signals()));
    }
    (deck, aims)
}

/// One binding, in a line, ending with what it will do rather than only what
/// it says.
fn describe(binding: &Binding, signals: &Signals) -> String {
    // **A published control is not on the bus**, and asking the bus about it
    // gets the answer for a name nothing measures — zero, which reads as a
    // binding that will do nothing. It is the operator's hand: confidence 1,
    // and the Set resolves it. See `karakuri_engine::binding::CONTROL_PREFIX`.
    let confidence = if binding.signal.starts_with(CONTROL_PREFIX) {
        1.0
    } else if binding.signal == NOISE_SIGNAL {
        signals.noise(&binding.noise.unwrap_or_default()).confidence
    } else {
        signals.sample(&binding.signal).confidence
    };
    let effect = if binding.signal.starts_with(CONTROL_PREFIX) {
        "the published control decides it outright".to_string()
    } else if confidence >= 1.0 {
        "the signal decides it outright".to_string()
    } else {
        format!(
            "it moves {:.0}% of the way and the param's own value holds the rest",
            confidence * 100.0
        )
    };
    format!(
        "bind {:?} {} <- {} through {} onto [{}, {}] — confidence {confidence:.2}, so {effect}",
        binding.layer,
        binding.key,
        binding.signal,
        binding.curve.name(),
        binding.range[0],
        binding.range[1],
    )
}

#[allow(clippy::too_many_arguments)]
fn build(
    gpu: &Gpu,
    l1s: &[karakuri_ir::typed::Checked],
    l2s: &[karakuri_ir::typed::Checked],
    // The cameras, in node order — see `Material::l3s`. Empty leaves the Set
    // looking from the built-in orbit.
    l3s: &[karakuri_ir::typed::Checked],
    // The fields, in node order — see `Material::fields`. Empty for a Set that
    // evaluates none.
    fields: &[karakuri_ir::typed::Checked],
    l4s: &[karakuri_ir::typed::Checked],
    layering: karakuri_engine::set::Layering,
    // What each node is called — see `Names`.
    names: &Names,
    // Which node fills each declared input slot — see `Args::edges`. Every one
    // the run was given, including any about another slot's Set, which the
    // engine passes over.
    edges: &[karakuri_engine::set::Edge],
    // One per entry in `l1s`, in the same order — see `capacities_for`.
    capacities: &[u32],
    // Set to `true` for each binding that attached, and left alone otherwise.
    // One flag per binding, and a caller building several slots ORs them: a
    // binding is worth describing if it attached *anywhere*.
    attached: &mut [bool],
    overrides: &[ParamWrite],
    bindings: &[Binding],
    published: &[karakuri_engine::set::Published],
    seed: u32,
    // One per entry in `l1s`, in the same order — see `salts_for`.
    salts: &[u32],
    camera: Option<karakuri_engine::camera::Orbit>,
    // Which renderer the Set comes up folded to — see `recorded_live`. `None`
    // leaves every input live, which is what `Set::build_many` builds and what
    // a Set nobody has selected in is.
    live: Option<u32>,
) -> Set {
    let deform: Vec<&karakuri_ir::typed::Checked> = l2s.iter().collect();
    let look: Vec<&karakuri_ir::typed::Checked> = l3s.iter().collect();
    let shapes: Vec<&karakuri_ir::typed::Checked> = fields.iter().collect();
    let draw: Vec<&karakuri_ir::typed::Checked> = l4s.iter().collect();
    // **Each source at the capacity it declares**, and `--capacity` overrides
    // all of them — one number cannot serve two L1s with different ranges.
    //
    // Asserted rather than zipped and hoped for: `zip` on a short list drops a
    // whole geometry, and a Set silently missing its second source is the same
    // picture as a Set that was never given one.
    assert_eq!(
        l1s.len(),
        capacities.len(),
        "one capacity per geometry source"
    );
    let sources: Vec<(&karakuri_ir::typed::Checked, u32)> =
        l1s.iter().zip(capacities.iter().copied()).collect();
    // **Resolved by the caller, never left to be filled in here.** The engine
    // derives a salt for a source nobody assigned one, and a run whose salts
    // were half assigned and half derived would be a run whose Set file records
    // numbers it was not using — so `salts_for` answers for every geometry and
    // this hands the whole answer over.
    assert_eq!(l1s.len(), salts.len(), "one salt per geometry source");
    let assigned: Vec<Option<u32>> = salts.iter().copied().map(Some).collect();
    match Set::build_many(
        &gpu.device,
        &gpu.queue,
        &sources,
        &deform,
        &look,
        &shapes,
        &draw,
        layering,
        seed,
        &assigned,
        karakuri_engine::set::Wiring {
            l1s: &names.l1s,
            l2s: &names.l2s,
            l3s: &names.l3s,
            l4s: &names.l4s,
            fields: &names.fields,
            edges,
        },
    ) {
        Ok(mut set) => {
            // **What the slot's nodes ended up called**, read off the Set
            // rather than worked out here: a name nobody wrote is derived in
            // `build_many`, and deriving it a second time to print it is the
            // shape this repository keeps finding wrong.
            eprintln!("  nodes: {}", set.node_names().join(", "));
            // **The `camera` record, into the node it is about.** It used to
            // be applied only to a Set whose files declared no L3, because an
            // L3 wrote the one camera state there was and an orbit assigned
            // beside it would have been overwritten before the first draw. The
            // orbit is its own node now — the last one, whatever else the Set
            // holds — so the record reaches it either way, and whether any
            // renderer draws from it is what an `edge` says rather than what
            // the file list happens to contain.
            if let Some(camera) = camera {
                set.camera = camera;
            }
            // **The `merge` record's selection, where the `camera` record's
            // six numbers go** — before the params and for their reason: a
            // value the file recorded is applied to the Set the file built,
            // once, in the one place that knows both. A Set comes up with
            // every input live, so leaving this out is a composited Set
            // loading back unselected, which is the half of a variant pool
            // that made saving one pointless.
            //
            // **Ineffective under `Overdraw`, and said rather than refused**,
            // which is `Set::select_renderer`'s own rule: a file that records
            // no `merge` records no selection either, so the only way to reach
            // this with an overdrawing Set is `--merge` left off a file that
            // has one — and `layering_for` decides that one line above.
            if let Some(at) = live {
                if !set.select_renderer(at as usize) {
                    eprintln!(
                        "  this set selects renderer {at} and has {} — every renderer is \
                         live",
                        l4s.len()
                    );
                }
            }
            for write in overrides {
                // **The refusal is the engine's sentence, printed rather than
                // reworded** — a `--param` and a `param` record reach the same
                // wall in the same words, which is
                // `docs/principles/0090-a-surface-offers-it-never-decides.md`.
                // Unreachable in a run today, because nothing grants a node's
                // authority yet and a Set nobody has spoken for lands
                // uniformly; the day a grant arrives, this line is what an
                // operator reads.
                match set.write_param(write) {
                    Ok(0) => eprintln!("  no parameter named `{}`, ignoring", write.key),
                    Ok(_) => {}
                    Err(refused) => eprintln!("  {refused}"),
                }
            }
            // After the overrides: a binding blends from the param's value, so
            // a `--param` on a bound param is the base of the blend rather
            // than a competitor for the write.
            // **Before the bindings**, because a macro is a binding whose source
            // is a published control: attaching one before the control existed
            // would be attaching it to a name nothing answers, and it would hold
            // its param where it found it for the rest of the run.
            for control in published {
                let name = control.name.clone();
                if let Err(e) = set.publish(control.clone()) {
                    eprintln!("  `{name}` is not published: {e}");
                }
            }
            for (at, binding) in bindings.iter().enumerate() {
                let (layer, key) = (binding.layer, binding.key.clone());
                match set.bind(binding.clone()) {
                    karakuri_engine::set::Bound::Yes => attached[at] = true,
                    karakuri_engine::set::Bound::NoSuchParam => {
                        eprintln!("  no {layer:?} parameter named `{key}` to bind, ignoring");
                    }
                    // **The other half of the same sentence.** This used to
                    // print the line above, which sends whoever reads it to
                    // look at the param — and the param is fine.
                    karakuri_engine::set::Bound::NoSuchControl => eprintln!(
                        "  `{}` is not published by this Set, so {layer:?} `{key}` is \
                         not bound",
                        binding.signal
                    ),
                }
            }
            set
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

struct App {
    args: Args,
    procs: Option<Vec<Material>>,
    /// Where each slot's files ended up, slot for slot beside `procs` — what a
    /// session's head is written from. See [`Placed`].
    placed: Vec<Vec<Placed>>,
    live: Option<Live>,
    /// The run's edit history, already holding what it started with. Handed to
    /// every slot's watcher when the deck is built — see [`history`].
    snapshots: Option<history::Shared>,
}

/// Put the session's grid where the tempo source says the shared grid is.
///
/// **The subtraction happens here and not in the source**, for the reason
/// `schedule_from` reads a transition's `from` end here: the two numbers — the
/// shared beat and this session's beat — are only both in hand at this moment.
/// A source that sent a shift would be sending a difference from a grid it
/// cannot see.
///
/// That subtraction is the whole of what a shared grid adds. A beat tracker can
/// find how fast beats go and where they are, and **cannot find which one is
/// beat one**; a number every peer agrees on can, and putting `beats()` onto
/// that number is what makes `bar` the room's bar rather than one counted from
/// whenever this program started.
///
/// **How far it is allowed to move is [`tempo_source::Source::correction`]'s**,
/// and that bound is not optional: the source is another program, from another
/// repository, released on its own schedule. This used to apply whatever
/// arrived, whole and instantly, which made a helper with a wrong clock able to
/// throw the grid thousands of beats.
///
/// Returns whether the grid is being followed, which is what takes the
/// authority to move it away from the beat tracker.
fn follow_tempo_source(
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
/// **Before the frame is rendered and never inside it.** The measured frame and
/// the tempo correction are latched here, exactly where `steps` is measured, so
/// that everything drawn this frame reads one set of values — two bindings
/// sampling `energy` in one frame have to get one answer, or the record saying
/// what this frame saw is a record of neither.
///
/// The session's signals are taken by value, given this frame's measurement,
/// and handed back. `Signals` is `Copy` and the copy carries the phase, so this
/// is not the "restart the session clock" that `Deck::set_signals` warns about
/// — it is the same clock with one frame's input attached.
///
/// **A free function rather than a method on `Live`**, for the reason [`Clock`]
/// is its own type: it is called from inside the closure that commits a frame,
/// which already holds the deck, so a `&mut self` here would borrow the whole
/// of `Live` a second time. Taking the three pieces it actually touches is also
/// a fair description of what it touches.
fn measure_audio(
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
/// **One function because there is one thing to say.** A replay governs too —
/// `Record::Residency` carries the request and never the effective level,
/// precisely so that the machine replaying re-derives it — and the first
/// version of that had its own smaller copy of these three loops, printing the
/// parked slots and not the summary. That is the shape this whole change exists
/// to remove: a second implementation that agrees until it does not.
fn report_governing(report: &karakuri_engine::governor::Report, why: &str) {
    eprintln!("{why}: {report}");
    for decision in report.parked() {
        eprintln!(
            "  slot {} parked, request held: {:?}",
            decision.slot, decision.reason
        );
    }
    for decision in &report.decisions {
        if decision.effective == Residency::Priming && decision.prime_one_in > 1 {
            eprintln!(
                "  slot {} priming at one step in {}",
                decision.slot, decision.prime_one_in
            );
        }
    }
}

/// The one measurement in the program: elapsed real time as a step count, which
/// is exactly what a `tick` record carries.
///
/// **Its own type so that reading it borrows only itself.** A frame acquires a
/// target, and only then commits — measuring the clock among other things — and
/// the committing work is a closure holding the deck and the recorder. A method
/// on `Live` would have borrowed all of it at once and the closure could not be
/// written. Splitting the clock out is what makes the ordering expressible.
///
/// **`steps` is told what time it is rather than asking.** One line of
/// plumbing, and it is what makes the thing this type exists to guarantee
/// checkable: that a frame which does not draw leaves its interval for the next
/// one instead of consuming it. With `Instant::now()` inside, a test could
/// state no elapsed time and could therefore assert nothing but tautologies —
/// which is exactly what the first test written against it did.
struct Clock {
    last: Instant,
    /// Fractional steps carried between frames, so a frame rate that does not
    /// divide the step rate still advances at the right average rate. The same
    /// accumulator shape as spawn quantisation, for the same reason.
    carry: f32,
    /// The last measured frame interval.
    interval: f32,
}

impl Clock {
    fn new(now: Instant) -> Clock {
        Clock {
            last: now,
            carry: 0.0,
            interval: DT,
        }
    }

    /// How many steps to advance by, called **once by every frame that runs**.
    ///
    /// It used to say *only by a frame that is going ahead*, because a frame
    /// that found no surface committed nothing and this was not reached — the
    /// interval it did not use was left for the next frame to count. A frame no
    /// longer belongs to a sink (see [`frame::compose`]), so every frame the
    /// loop runs reads the clock and there is nothing to carry between frames.
    ///
    /// `last` still moves here and nowhere else, and that is still what makes a
    /// gap survive: the frame loop not running at all — a paused event loop, a
    /// window the system stopped asking to redraw — is counted whole by the
    /// next frame, up to `MAX_STEPS`.
    fn steps(&mut self, now: Instant) -> u8 {
        let elapsed = now.duration_since(self.last).as_secs_f32();
        self.last = now;
        // Kept because the output lag a beat correction leads by starts with
        // the frame queue, which is a number of *frames* — and the frame rate
        // is the display's, not `dt`'s. See `audio::Audio::output_lag`.
        self.interval = elapsed;

        self.carry += elapsed / DT;
        let whole = self.carry.floor();
        self.carry -= whole;
        (whole as u32).min(u32::from(MAX_STEPS)) as u8
    }

    fn interval(&self) -> f32 {
        self.interval
    }
}

/// **Every save still in flight, collected until they are all in or `deadline`
/// passes.**
///
/// A free function over the channel rather than a loop inside
/// [`Live::awaited_saves`], so that the bound — which is the whole of what makes
/// waiting at the end of a run safe rather than a way to hang on a bad disk —
/// can be checked without a window and a GPU. That is the same reason
/// [`live_sources`] is a free function.
fn drained_saves(
    rx: &std::sync::mpsc::Receiver<Saved>,
    in_flight: usize,
    deadline: Instant,
) -> Vec<Saved> {
    let mut landed = Vec::new();
    while landed.len() < in_flight {
        let Some(left) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        match rx.recv_timeout(left) {
            Ok(saved) => landed.push(saved),
            // Timed out, or every sender is gone and nothing more can arrive.
            // Either way there is nothing left to wait for.
            Err(_) => break,
        }
    }
    landed
}

/// **One live save, from the frame that asked for it to the file on disk.**
struct Save {
    slot: usize,
    /// **Whose act this save is**, which decides the directory it lands in and
    /// is decided at the call site — see [`Live::save_set`] and
    /// [`karakuri_environment::Asked`].
    asked: Asked,
    id: String,
    /// The store root, not an open store: opening it creates directories, which
    /// is I/O, which belongs on the thread below rather than on a frame.
    root: PathBuf,
    sources: Sources,
    /// **What the file will say, with `nodes` still empty.** The nodes are the
    /// one part of a Set file that needs a store — a hash per source — so they
    /// are filled in where one is opened and never here.
    ///
    /// A half-built value crossing a thread boundary is worth a sentence,
    /// because the alternative was considered and is worse: a second struct
    /// holding "the other six fields" is a type whose whole content is which
    /// field it is missing, and it would have to be kept in step with
    /// `setfile::Owned` by hand forever.
    values: setfile::Owned,
}

impl Save {
    /// Write it. **Everything here is off the render thread**: opening a store
    /// creates directories, and the Set file itself is written and renamed into
    /// place.
    fn run(self) -> Result<(), String> {
        let Save {
            asked,
            id,
            root,
            sources,
            mut values,
            ..
        } = self;
        let store = karakuri_store::store::Store::open(&root)
            .map_err(|e| format!("store `{}`: {e}", root.display()))?;
        // **Before the file that references them**, which is what
        // [`setfile::Node`] carrying a hash asks of every caller: the writer
        // cannot check that a hash resolves without reading the store back, so
        // putting them is the caller's promise. See [`Sources::into_nodes`].
        values.nodes = sources.into_nodes(&store)?;
        setfile::save(&store, asked, &id, values.saving())
    }
}

/// What a live save came back with, at the frame it arrives.
struct Saved {
    slot: usize,
    /// Carried through so the sentence at the end names the right directory:
    /// the library's line tells an operator how to load it back, and the
    /// sandbox's cannot, because nothing loads one.
    asked: Asked,
    id: String,
    /// `Ok` and the file is on disk under `id`. **A failure is printed and no
    /// record is written**: a stream saying a save happened when the disk
    /// refused is exactly the shape of lie this codebase spends its comments
    /// refusing.
    outcome: Result<(), String>,
    /// Where a client that asked for this save is waiting, and `None` when a
    /// hand pressed `k`.
    ///
    /// **It rides the save rather than being looked up when the outcome lands.**
    /// A map from an id to whoever asked would be a second place that knows
    /// which save is which, and the outcome already carries everything needed to
    /// find its way home.
    reply: Option<mcp::Reply>,
}

/// A save that will not happen, to the terminal and to whoever asked if it was
/// not a hand.
///
/// **One sentence and one home.** Every refusal here reaches two audiences now,
/// and the way that goes wrong is a copy of the words for the second one — which
/// is free to be right on the day it is written and wrong at the next
/// correction. The wording of the refusal below has already needed one.
fn refused(reply: Option<mcp::Reply>, said: String) {
    eprintln!("{said}");
    if let Some(reply) = reply {
        reply.settled(Err(said));
    }
}

/// Whether `at` names a renderer of a slot that draws with `count` of them, and
/// the sentence if it does not. [`slot_in_range`]'s companion, returning the
/// refusal rather than a bool because both callers print it.
fn renderer_in_range(slot: usize, at: usize, count: usize) -> Result<(), String> {
    if at < count {
        Ok(())
    } else {
        Err(no_such_renderer(slot, at, count))
    }
}

struct Live {
    window: Arc<Window>,
    gpu: Gpu,
    /// Where a composed frame goes. The window is **a** sink rather than *the*
    /// output — see `docs/plugins.md`, where the others hang.
    sink: frame::WindowSink,
    present: Present,
    /// Every Set, whatever is being built to replace any of them, and the mix.
    /// Without `--watch` every slot is a `HotSwap::fixed` and there is no
    /// worker at all, so the frame loop below is the same code either way.
    deck: Deck,
    look: Look,
    /// The slot the gain keys act on. There is no on-screen UI, so this is
    /// printed on every change and marked in the status line.
    focus: usize,
    clock: Clock,
    /// The audio input, the beat lock, and the operator's latency offset.
    /// `None` without `--audio-in`, and then nothing in the frame path below
    /// changes at all — which is the property the whole slice is about.
    audio: Option<audio::Audio>,
    /// The control surface, when `--midi-in` asked for one. Every operation it
    /// produces ends in the same record a key press ends in — see
    /// [`crate::midi`] — so nothing in the frame path below changes at all when
    /// this is `None`.
    midi: Option<midi::Surface>,
    /// The tempo source, when `--tempo-source` asked for one. `None` and
    /// nothing in the frame path below changes at all — the grid comes from
    /// `--bpm`, the tracker and the tap keys, exactly as it did before this
    /// existed. That is the same property `audio` and `midi` have and it is
    /// the one worth keeping.
    tempo_source: Option<tempo_source::Source>,
    /// What each slot's watcher built, by build id, until the swap that build
    /// produced lands. **Not a log**: an entry is taken when its build lands or
    /// dropped when a newer one supersedes it.
    rebuilds: Option<std::sync::mpsc::Receiver<watch::Built>>,
    pending_builds: std::collections::HashMap<u64, watch::Built>,
    /// **What each slot is running**, and what a rollback of it would bring
    /// back. Seeded before the first frame from the text the compile read, so
    /// there is exactly one way to answer the question a live save asks — see
    /// [`Running`].
    running: Running,
    /// **Where each slot's files were at launch**, one entry per node, in the
    /// order they were spelled.
    ///
    /// **The names, and the bytes each node was compiled from.** A name belongs
    /// to the use rather than to the procedure, so `--set veil=shell.kir` is a
    /// fact about the command line that no rebuild restates and no hash
    /// carries, and `Live::save_set` zips these onto the hashes by position.
    ///
    /// **Not the paths, which is the distinction that matters.** This used to
    /// answer "what is this slot running" by re-reading `named.path`, and that
    /// was a second answer to a question [`Running`] already held. What is here
    /// now is [`Placed::source`] — the text the compile read — and it is not a
    /// second answer but the *first*: every hash in `Running` was derived from
    /// it, and a save hands the same buffer to the store so the file it writes
    /// resolves. There is one derivation, and this is where its input lives.
    ///
    /// Empty for a slot filled straight from a Set file without `--watch` or
    /// `--mcp`, which has no files behind it at all — see where `placed` is
    /// built.
    startup: Vec<Vec<Placed>>,
    /// The Set file this run was loaded from, for the one refusal that has to
    /// name it: a slot filled from a file with nothing watching it has no
    /// sources to save, and an operator asking why is owed the id and the flag
    /// that would change the answer.
    loaded_set: Option<String>,
    /// **Which node fills each declared input slot**, for the whole run.
    ///
    /// Read from the arguments rather than from the live Set, which is the one
    /// place `Live::save_set` does that and needs its reason. An edge is
    /// consumed where a Set is *built* and is not kept on it, so there is
    /// nothing to read back. So this is a copy of a value, not a second copy
    /// of a rule, which is the distinction `saving_capacities` was fixed over.
    ///
    /// **It moves during a run, and this is where it moves.** That sentence
    /// used to read *no key and no MCP tool rewires a `uses` slot*; `wire_input`
    /// does, and [`rewired`] is what it reaches. There is still no second
    /// answer to what the run is wired with — this list is the one, a rewiring
    /// replaces one entry of it and hands the whole of it to the slot's watcher
    /// through [`Aiming`], so the list a rebuild restates and the list a save
    /// records stay one list.
    ///
    /// **One list for the run and not one per slot**, which is what makes the
    /// key `(node, slot)` and not `(deck, node, slot)` — see [`rewired`], and
    /// `Wiring::edges` for why an edge naming a node a Set has not got is
    /// simply passed over.
    ///
    /// **A limit worth naming**: a rewiring re-aims only the slot the request
    /// named, so any *other* watcher goes on restating the list it was last
    /// aimed with until it is itself re-aimed. That is invisible unless two
    /// slots hold nodes of the same name, which is also the only case where one
    /// entry in this list was ever about two Sets. Re-aiming every watcher
    /// instead would recompile the whole deck for one edge, which is a far
    /// louder wrong answer.
    edges: Vec<karakuri_engine::set::Edge>,
    /// **Where each slot's watcher can be re-pointed**, one entry per slot and
    /// `None` for a slot with no watcher — a run without `--watch`, and every
    /// `HotSwap::fixed`. See [`Aiming`]: this is how an edge written over MCP
    /// reaches the thing that rebuilds with it.
    aims: Vec<Option<Aiming>>,
    /// The store root a live save writes into. The *root* and not an open
    /// store: every part of a save that touches a disk happens on the thread
    /// that does it — see [`Save::run`].
    store_root: PathBuf,
    /// **Where a save reports back.** One thread per save writes into the
    /// sender's clone; the frame loop drains the receiver, which is the shape
    /// `rebuilds` already has and for the same reason: an outcome arrives when
    /// it arrives, and a frame must not wait for it.
    save_tx: std::sync::mpsc::Sender<Saved>,
    saves: std::sync::mpsc::Receiver<Saved>,
    /// **How many saves have been started and not yet reported back.** The
    /// threads are detached, so this is the only thing that knows a file is
    /// still being written — see [`Live::awaited_saves`], which is why anything
    /// counts them at all.
    saves_in_flight: usize,
    /// The MCP server's half of the channel, when `--mcp` asked for one. Told
    /// what the swap machinery said, and nothing else — see [`crate::mcp`].
    mcp: Option<mcp::Reporter>,
    /// Scratch for [`midi::Surface::take`], owned so the frame path allocates
    /// nothing. Empty on every frame nothing was touched.
    ///
    /// Nothing a map line can name carries a heap payload — every one of them
    /// is scalars — so a fader sweep reuses this buffer and touches no
    /// allocator, which is what the render-thread rule asks of it.
    operations: Vec<Operation>,
    /// The musical grid a scheduled fade starts on — see [`QUANTA`]. State on
    /// the operator rather than in the record: what reaches the stream is the
    /// resolved beat count, so this is a setting for the hand and not for the
    /// timeline.
    quantum: f64,
    /// How long a scheduled fade lasts, in beats. Same reasoning.
    fade_beats: f64,
    /// The shape the next wipe uses, and which way it runs. Not a slot's mask:
    /// this is what `c` will *give* a slot, where the slot's own is deck state
    /// and travels in the record stream.
    mask_kind: MaskKind,
    mask_angle: f32,
    /// When the session started, so a tap has an origin to be measured from.
    started: Instant,
    status_at: Instant,
    frames_since_status: u32,
    /// Writes the timeline, when `--record-session` asked for one. The frame
    /// path pushes into it and never blocks or allocates — see
    /// [`crate::session`].
    recorder: Option<session::Recorder>,
    /// Which demonstration is running and how far into its script, or `None`
    /// when `--demo` was not given and nothing drives itself.
    demo: Option<(Demo, usize)>,
    /// When the current pass through [`DEMO_SCRIPT`] started. The script loops,
    /// so this is not [`Live::started`]: that one is the session's origin and a
    /// tap is measured from it.
    demo_started: Instant,
    /// Reused by the status line. Printing at all on this thread means locking
    /// stderr, but there is no reason for it to mean a fresh allocation twice a
    /// second as well.
    status: String,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Some(procs) = self.procs.take() else {
            return;
        };

        let attrs = Window::default_attributes()
            .with_title("Karakuri")
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.args.size.0,
                self.args.size.1,
            ));
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        let size = window.inner_size();

        let instance = Gpu::instance();
        let surface = instance.create_surface(window.clone()).expect("surface");
        let gpu = pollster::block_on(Gpu::from_instance(instance, Some(&surface))).expect("gpu");

        let caps = surface.get_capabilities(&gpu.adapter);
        // sRGB encoding happens once, at final output: pick a surface format
        // that carries the transfer function so the hardware does it on write.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            // The format above already carries the transfer function, and
            // `Auto` is the one value that leaves the presentation engine
            // interpreting the swapchain exactly as it always has: sRGB for an
            // `*Srgb` format, and never a wide-gamut or HDR space picked
            // behind the pipeline's back. See P-0064 — sRGB is encoded once,
            // at final output, and that is here.
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width.max(1),
            height: size.height.max(1),
            // **Chosen, not taken.** This was `caps.present_modes[0]`, which is
            // whatever order the backend happened to list — so the pacing of a
            // run was a property of the driver, invisible and unsettable, and
            // the same session ran differently on two machines with nothing
            // saying so. `Fifo` is supported on every platform and is
            // `PresentMode`'s own default, so naming it costs nothing and makes
            // the answer the same everywhere. It is also the right answer for
            // this output: tearing across a projected image is worse than a
            // frame of latency.
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&gpu.device, &config);

        // **The canvas, not the window.** These two were the same number until
        // the window was named a preview: what is drawn is the session's and
        // what it is looked at through is not, so a window that opened at an
        // odd size no longer decides what a run renders — and dragging one no
        // longer reallocates every slot's target on the render thread.
        let (canvas_w, canvas_h) = self.args.canvas;
        check_canvas(&gpu.device, canvas_w, canvas_h);
        let present = Present::new(&gpu.device, format, canvas_w, canvas_h);
        // **Opened before the deck, because a watcher needs it.** A rebuilt
        // procedure has to reach the store from the worker thread that built
        // it; by the time the swap lands on a frame, the file may have changed
        // again and the render thread is the wrong place for file I/O.
        //
        // **Whenever the run is editable**, rather than only when a session is
        // being recorded. Two things read these hashes now — the `procedure`
        // records and `Live::save_set` — and the second is wanted in the
        // ordinary `--watch` case, which records nothing. See
        // `watch::Watch::stored`.
        let (rebuilds, rebuild_rx) = match editable(&self.args) {
            true => {
                let store = std::sync::Arc::new(open_store(&self.args));
                let (tx, rx) = std::sync::mpsc::channel();
                (Some((store, tx)), Some(rx))
            }
            false => (None, None),
        };
        let (mut deck, aims) = build_deck(
            &gpu,
            &procs,
            &self.args,
            self.args.watch,
            true,
            canvas_w,
            canvas_h,
            rebuilds,
            self.snapshots.clone(),
        );
        // Here and nowhere else: before the first frame, where the stall it
        // costs is free. Nothing else measures the Sets a run starts with —
        // only the build worker measures, and at startup it has built nothing
        // — and **one unmeasured live slot makes the whole deck's committed
        // cost unknown**, which parks every priming request there is with
        // `Reason::CommittedUnknown`. Without this call the governor below is
        // an elaborate way of saying no.
        deck.measure_slots(&gpu.device, &gpu.queue);

        eprintln!(
            "running: {} slot{} of {} elements at {:.1} bpm on {}",
            deck.slot_count(),
            if deck.slot_count() == 1 { "" } else { "s" },
            self.args.capacity,
            self.args.bpm,
            gpu.adapter.get_info().name
        );
        // What the deck costs and what it is allowed, printed once at startup
        // so the numbers a park is later explained by are not the first the
        // operator sees. Every millisecond in that line is a cold Set measured
        // at a reference resolution — comparable between slots, not a
        // prediction of this machine's frame time. The line says which clock
        // it came off, which is the part that changes between machines.
        eprintln!("  {}", deck.govern());
        if self.args.watch {
            for (slot, (l1, l4s)) in self.args.sets.iter().enumerate() {
                eprintln!(
                    "  watching slot {slot}: {} and {} — a save recompiles that slot in the \
                     background and swaps it when ready, budget {:.1} ms",
                    l1.path.display(),
                    l4s.iter()
                        .map(|p| p.path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(" and "),
                    self.args.budget_ms
                );
            }
        }
        // Opened before the first frame and never on it. A failure here is
        // fatal on purpose: `--audio-in` was asked for, and a run that quietly
        // continued without it would look exactly like a run whose bindings
        // are all at a tenth effect for some other reason.
        let audio = match &self.args.audio_in {
            Some(selector) => {
                match audio::Audio::open(selector, self.args.latency_offset_ms, DT, self.args.bpm) {
                    Ok(audio) => {
                        eprintln!(
                            "audio in: {} at {} Hz — energy, onset and band0..7 are measured now, \
                             and the beat corrects the oscillator. output offset {:.0} ms (o/p)",
                            audio.description(),
                            audio.sample_rate(),
                            audio.latency_offset_ms()
                        );
                        Some(audio)
                    }
                    Err(e) => {
                        eprintln!("karakuri-cli: {e}");
                        std::process::exit(2);
                    }
                }
            }
            None => None,
        };

        // Opened on the same terms as the audio device and for the same
        // reason: `--midi-in` was asked for, and a run that quietly continued
        // without it would look exactly like a run whose surface is plugged in
        // and doing nothing.
        let midi = match &self.args.midi_in {
            Some(selector) => match midi::Surface::open(selector, self.args.midi_map.as_deref()) {
                Ok(surface) => Some(surface),
                Err(e) => {
                    eprintln!("karakuri-cli: {e}");
                    std::process::exit(2);
                }
            },
            None => None,
        };

        // Third of the same kind. Fatal for the same reason: `--tempo-source`
        // was asked for, and a run that quietly went on following its own grid
        // would look exactly like one whose source is attached and agreeing.
        let tempo_source = match &self.args.tempo_source {
            Some(command) => match tempo_source::Source::open(command) {
                Ok(source) => {
                    eprintln!(
                        "tempo source: `{}` — the grid follows its beat, and `bar` is the \
                         room's rather than one counted from when this started",
                        source.name()
                    );
                    Some(source)
                }
                Err(e) => {
                    eprintln!("karakuri-cli: tempo source: {e}");
                    std::process::exit(2);
                }
            },
            None => None,
        };

        // Fourth of the same kind. Fatal for the same reason the others are:
        // `--mcp` was asked for, and a run that went on without it would look
        // exactly like one whose client is connected and idle.
        let mcp = match self.args.mcp {
            Some(port) => {
                let slots = mcp::Slots(
                    self.args
                        .sets
                        .iter()
                        .map(|(l1, l4s)| {
                            (
                                l1.path.clone(),
                                l4s.iter().map(|n| n.path.clone()).collect(),
                            )
                        })
                        .collect(),
                );
                // The store this run was given, not a second answer to where
                // the library is: `read_set` reads a saved Set and its cards
                // out of the same root `--save-set`, `--load-set` and the `k`
                // key write into.
                // **Closed, all four classes**, which is the state ADR-0235
                // says a run starts in. It is handed in rather than decided
                // inside the server: what a model may reach is the operator's
                // to say (P-0094), and a constant compiled into the server is
                // the one place it must not be said. **Nothing writes it
                // yet** — the bay-head toggles are the console's — so every
                // closed class stays closed for the whole run, and the handle
                // is what the toggles will hold the other end of.
                let opening = karakuri_environment::Opening::closed();
                match mcp::serve(
                    port,
                    slots,
                    self.args.store.clone(),
                    self.args.watch,
                    opening,
                ) {
                    Ok(reporter) => {
                        // The port bound rather than the one asked for: `--mcp 0`
                        // takes an ephemeral one, and printing the 0 would name
                        // a port that is not the port.
                        let port = reporter.port();
                        eprintln!(
                            "mcp: 127.0.0.1:{port} — a client can read and rewrite a slot's \
                             procedure{}",
                            if self.args.watch {
                                ""
                            } else {
                                ", but without --watch nothing will pick a write up"
                            }
                        );
                        Some(reporter)
                    }
                    Err(e) => {
                        eprintln!("karakuri-cli: mcp: {e}");
                        std::process::exit(2);
                    }
                }
            }
            None => None,
        };

        eprint!("\n{BINDINGS}\n");

        // Opened before the first frame and never on one: it creates a file
        // and spawns a thread. A failure is fatal because `--record-session`
        // was asked for, and a run that quietly continued without it would be
        // a performance nobody can replay and nothing saying so.
        let recorder = match &self.args.record_session {
            Some(id) => {
                let store = open_store(&self.args);
                let geometries = self
                    .procs
                    .as_ref()
                    .and_then(|procs| procs.first())
                    .map(|m| m.l1s.as_slice())
                    .unwrap_or(&[]);
                let head = session_head(&self.args, &self.placed, geometries, &store, id);
                seed_store_for_replay(&store, &self.placed);
                match session::Recorder::open(&store, id, &head) {
                    Ok(recorder) => {
                        eprintln!(
                            "recording session `{id}` — {} record{} of material at its head, \
                             so `--replay {id}` needs nothing else",
                            head.len(),
                            if head.len() == 1 { "" } else { "s" }
                        );
                        Some(recorder)
                    }
                    Err(e) => {
                        eprintln!("karakuri-cli: {e}");
                        std::process::exit(1);
                    }
                }
            }
            None => None,
        };

        let slot_count = deck.slot_count();
        // Opened before the first frame like every other channel here, and
        // never on one. Nothing is spawned until a key is pressed.
        let (save_tx, saves) = std::sync::mpsc::channel();
        // **Before the first frame, and for every windowed run.** This is what
        // makes "what is this slot running" a hash from the outset rather than
        // a path some later state contradicts. No I/O and nothing that can
        // fail: the addresses come off the bytes the compile read — see
        // [`Running::at_launch`].
        let running = Running::at_launch(&self.placed, slot_count);
        let live = Live {
            window,
            gpu,
            sink: frame::WindowSink::new(surface, config),
            present,
            deck,
            look: self.args.look,
            focus: 0,
            clock: Clock::new(Instant::now()),
            audio,
            midi,
            tempo_source,
            rebuilds: rebuild_rx,
            pending_builds: std::collections::HashMap::new(),
            running,
            // **Cloned rather than moved**, because `self.placed` is what
            // `session_head` above was handed and `App` outlives this. It is a
            // handful of paths per slot, once, at startup.
            startup: self.placed.clone(),
            loaded_set: self.args.load_set.clone(),
            edges: self.args.edges.clone(),
            aims,
            store_root: self.args.store.clone(),
            save_tx,
            saves,
            saves_in_flight: 0,
            mcp,
            operations: Vec::new(),
            quantum: QUANTA[0].0,
            fade_beats: FADE_BEATS[0],
            mask_kind: MASK_SHAPES[0].0,
            mask_angle: MASK_SHAPES[0].1,
            started: Instant::now(),
            status_at: Instant::now(),
            frames_since_status: 0,
            status: String::with_capacity(256),
            demo: self.args.demo.map(|d| (d, 0)),
            recorder,
            demo_started: Instant::now(),
        };
        // Through a record at startup too, on the same terms as every later
        // change: `--tonemap` and `--exposure` are an operator's choices rather
        // than the engine's defaults, so a session that did not carry them
        // would replay under whatever look the next build happens to default
        // to. This is also the first thing that decodes one, so a `look` this
        // build cannot obey is reported before a frame is drawn.
        let mut live = live;
        // **Before the look, and once.** A replay reads this out of the stream
        // to size everything it allocates, so it has to be there before any
        // record that describes a frame — and there is deliberately no second
        // writer anywhere, which is what "fixed for the run" means in practice.
        live.record(mix::canvas_record(canvas_w, canvas_h));
        live.record(mix::look_record(&self.args.look));
        self.live = Some(live);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => live.resize(size.width, size.height),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && live.key(&event.logical_key) {
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                live.frame();
                live.window.request_redraw();
            }
            _ => {}
        }
    }

    /// The one place a stall is welcome: every frame has been rendered and the
    /// run is over, which is exactly the case `Set::live_count` is documented
    /// to be for.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(live) = &mut self.live {
            // Before the counts, because it ends a thread and flushes a file
            // and those are the things worth knowing failed.
            // Before the counts and before the recorder: it is another process
            // and leaving it running would outlive the window that started it.
            if let Some(source) = live.tempo_source.take() {
                source.close();
            }
            // **Before the recorder is finished**, so a save that landed after
            // the last frame is still in the stream it belongs to — including
            // one that was still being written when the window closed, which is
            // what the bounded wait is for. See [`Live::awaited_saves`].
            live.awaited_saves();
            if let Some(recorder) = live.recorder.take() {
                match recorder.finish() {
                    Ok(w) => {
                        eprintln!("session: {} records written", w.records);
                        // **Named rather than counted quietly**, and named
                        // apart: a lost batch is a second of everything and a
                        // lost audio frame is one frame's measurement, and an
                        // operator deciding what to do about a stream needs to
                        // know which it has.
                        if w.dropped_batches > 0 {
                            eprintln!(
                                "  {} batch{} lost because the disk could not keep up — \
                                 the stream has gaps",
                                w.dropped_batches,
                                if w.dropped_batches == 1 { "" } else { "es" }
                            );
                        }
                        if w.dropped_audio > 0 {
                            eprintln!(
                                "  {} frame{} of audio not recorded — those frames replay \
                                 with the bus's invented values rather than what was heard",
                                w.dropped_audio,
                                if w.dropped_audio == 1 { "" } else { "s" }
                            );
                        }
                    }
                    Err(e) => eprintln!("session: {e}"),
                }
            }
            report_live_counts(&live.gpu, &live.deck);
        }
    }
}

impl Live {
    /// The **window** changed size. Nothing that is rendered changes.
    ///
    /// This used to resize the HDR target and every deck slot as well, because
    /// the window's size *was* the canvas. Two things came of that, and both
    /// are gone with it: dragging a window reallocated every slot's target once
    /// per frame of the drag — a GPU allocation on the render thread, which is
    /// the one thing this engine's frame path forbids — and what a run rendered
    /// depended on how big its window happened to be, so the same session
    /// replayed at a different size with nothing saying which was the
    /// performance. All that is left here is the swapchain, which has to follow
    /// the window because it *is* the window.
    fn resize(&mut self, width: u32, height: u32) {
        self.sink.resize(&self.gpu.device, width, height);
    }

    /// Resize the window so the canvas lands in it one texel to one texel.
    ///
    /// The preview is fitted, so an OBS window capture of it would otherwise
    /// pick up the bars and a scale — and a capture that is neither the canvas
    /// nor a clean crop of it is worse than useless downstream. After this the
    /// window contains the canvas exactly, and `letterbox` becomes the identity.
    ///
    /// **A request, not a guarantee, and the difference is printed.** A 1080-tall
    /// canvas cannot get a 1080-tall content window on a 1080-tall display —
    /// there is a menu bar or a taskbar in the way — so the manager clamps it,
    /// and an operator setting up a capture has to be told that rather than
    /// told "1:1".
    ///
    /// `request_inner_size` returns the granted size **immediately** on the
    /// platforms where the manager decides and `None` where a `Resized` event
    /// will follow. Taking the returned value matters on the first kind: no
    /// event arrives, so nothing else would ever reconfigure the swapchain, and
    /// a swapchain that disagrees with its window does not fail — `set_viewport`
    /// is not validated against the attachment — it just draws the wrong
    /// picture, silently.
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
    /// Wall clock rather than frame count, because what is being demonstrated
    /// is a performance and a performance happens in seconds. That makes the
    /// demo *not* reproducible frame for frame, which is fine and is worth
    /// saying: it is a thing to look at, not a thing to diff. Everything it
    /// presses goes through [`Live::key`], so it can do nothing a person at the
    /// keyboard could not.
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

    /// **Whatever the control surface did since the last frame**, as the same
    /// operations a key press and a console fader name.
    ///
    /// The whole of the MIDI connection, and there is almost nothing in it:
    /// a mapped message *is* an [`Operation`], so this hands each one to
    /// [`Live::operate`] and that is the connection. A surface can do nothing
    /// a key cannot because both end in the same record, and a session
    /// recorded from one replays with neither attached.
    ///
    /// **This used to be a match over eight `Action`s claiming that a control
    /// added to one and not the other does not compile.** Against a
    /// fifty-variant vocabulary that claim would be false — a router arm
    /// nobody wrote is a wildcard nobody notices. The guarantee is now where
    /// it is true: `karakuri_operation_record::written` is one exhaustive
    /// match over all fifty, so an operation nobody has said what to do
    /// with stops the build there.
    ///
    /// **[`Operation::TapBeat`] is handled here and it is the only one**, for
    /// a reason that is visible rather than incidental: a tap moves the beat
    /// tracker rather than writing a value, `written` answers
    /// `Owed::NotSettled` for it, and `Live::operate` would print that gap
    /// instead of tapping. `Live::tap` is what owns the tracker and what the
    /// `b` key reaches, so `note -> tap` goes on doing exactly what it did.
    /// The day the record a tap owes is settled, this arm is what goes.
    ///
    /// **Nothing here prints.** The old arms ended in `set_gain` and its
    /// neighbours, each of which reports what it did — which on a fader sweep
    /// is an `eprintln!` per MIDI message inside a frame, several hundred a
    /// second, and is the blocking write per message `crate::midi`'s own
    /// "once per control" rule exists to prevent. What a surface moved is read
    /// back from the deck (`s`), not narrated per message.
    ///
    /// Before the tick, so a fader move lands on the frame it arrived for
    /// rather than the one after — the same placement `run_demo` has, and for
    /// the same reason.
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
        surface.take(self.deck.slot_count(), &mut operations);
        for operation in &operations {
            match operation {
                Operation::TapBeat => self.tap(),
                other => self.operate(other),
            }
        }
        self.operations = operations;
    }

    /// **What a model has asked for since the last frame.**
    ///
    /// Beside [`Live::run_surface`] and on the same terms: a surface is polled
    /// at the top of a frame and every request it produces ends in the method a
    /// key press ends in. That is what makes `--mcp` a third pair of hands
    /// rather than a second way to do anything.
    ///
    /// **Collected out of the borrow before any of it is acted on**, exactly as
    /// the MIDI operations are, because every arm below takes `&mut self`. Nothing
    /// is allocated on a frame that was asked for nothing: collecting an empty
    /// iterator makes no allocation.
    ///
    /// Here rather than beside the swap drain below `frame::compose`, which was
    /// the other candidate: a client asking to keep what is playing should not
    /// be waiting on a swapchain, and nothing a request reaches needs the GPU.
    /// When that was written a frame with no surface returned before the drain,
    /// so it *was* waiting on one; a frame no longer returns early at all, and
    /// being above `frame::compose` is what the argument was always about.
    ///
    /// **[`Live::finished_saves`] is here for the same reason and used to be
    /// down there**, which meant this argument was made and then half applied:
    /// the request was taken above the early returns and its *answer* was
    /// withheld below them. See the comment at the head of [`Live::frame`].
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
        for request in asked {
            self.save_set(Asked::Model, request.slot, request.id, Some(request.reply));
        }
        self.rewire(wires);
    }

    /// **Every edge asked for since the last frame, written and answered here,
    /// on this frame.**
    ///
    /// The decisions are [`rewired`]'s and are written there, because none of
    /// them needs a `Live`. What is here is the two things that do: the deck's
    /// own slot count, which is the only thing that knows how many slots there
    /// are, and the answer going back to whoever asked.
    ///
    /// **Answered once, at the frame it was applied on**, which is
    /// [`mcp::WireRequest`]'s third point. Not at the swap: what the *build*
    /// made of the edge is `swap_outcome`'s answer, as it is for every other
    /// rebuild, and a tool that waited for thirty judged frames would hold a
    /// connection open across a transition.
    ///
    /// **One sentence for both audiences**, which is [`refused`]'s rule: what
    /// the terminal is told and what the client is handed are the same words,
    /// so the second cannot be right on the day it is written and wrong at the
    /// next correction.
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
    /// `karakuri_operation_record::written` converts. **A keyboard is not that
    /// shape**, and this is the survey rather than an intention: of the
    /// thirty-nine keys below, twenty-one name an operation whose record
    /// converts, three name one whose record is owed, twelve name one that
    /// writes no record at all, and three name nothing in the vocabulary.
    ///
    /// - **Its record converts, so it goes through [`Live::operate`].**
    ///   `space` and `w` (`SetResidency`), `[`, `]` and `\` (`SetGain`),
    ///   `;` and `'` (`SetOpacity`), `m` (`SetBlendMode`),
    ///   `t` (`SetTonemap`), `-`, `=` and the backquote (`SetExposure`), `u`
    ///   and `i` (`ScrubDeck`), `y` (`SetSync`), `f` and `g` (`FadeDeck`),
    ///   `x` (`Crossfade`), `r` (`SelectRenderer`), `c` (`Wipe`). A key and a
    ///   mapped pad reach the deck by **one
    ///   derivation**, which is the whole of why the two surfaces cannot drift.
    ///   Which way to step is still the keyboard's — a cycle and a nudge are
    ///   translations a surface makes, never operations
    ///   (`docs/principles/0090-a-surface-offers-it-never-decides.md`).
    /// - **Its record is owed, so it keeps its present path** — each with the
    ///   reason written at the function it ends in: `b` ([`Live::tap`]), `,`
    ///   and `.` ([`Live::shift_octave`]). All three
    ///   operations answer `Owed::NotSettled`, and the day one is settled its
    ///   function is what goes — the promise `run_surface`'s `TapBeat` arm
    ///   already carries. Routing one through `operate` today would print the
    ///   gap where the gesture used to happen, which is the one thing a change
    ///   of route may not do.
    ///
    ///   **It was eight, and the five that left are what the promise looks
    ///   like kept three times.** `Operation::SetSync` was owed for the
    ///   engine's anchor clamp; the clamp turned out to be the identity on
    ///   every tempo an oscillator can report, the conversion took a session
    ///   tempo as a reading, and [`Live::cycle_sync`] moved to `operate` with
    ///   its record-building deleted rather than kept in step. `FadeDeck`,
    ///   `Crossfade` and `SelectRenderer` went the same way and took a whole
    ///   function with them: the quantum and the length `n` and `j` cycle are
    ///   **this surface's**, `Current::transition` is where they are handed
    ///   over, and `Live::fade_slot` is gone rather than kept beside the
    ///   conversion. **`c` was the one that stayed and is the fifth to go**,
    ///   because what a wipe carried beyond a fade turned out to be unassigned
    ///   rather than missing too: the shape its front takes is the third of
    ///   the settings `z`, `n` and `j` write and travels with the other two,
    ///   and the soft edge is read off the mask on the deck being wiped in.
    ///   [`Live::wipe`] keeps only the two refusals and the line it prints.
    ///
    ///   **What is left is the beat tracker, and it is a different shape.** A
    ///   tap and an octave shift do not want a reading nobody hands over; they
    ///   want the beat lock's answer, which no value a surface holds
    ///   determines — see [`Live::tap`].
    /// - **It writes no record, and this surface is what has to perform it.**
    ///   `0`–`3` (`SelectDeck`), `z`, `n` and `j` (`SetTransition`), `a`
    ///   (`SizeWindow`), `k` (`SaveSet`), `o` and `p` (`SetLatencyOffset`),
    ///   `esc` (`Quit`). **These cannot route through `operate` either, and
    ///   that is a fact about `Silent` rather than an omission**: `operate`
    ///   turns an operation into the records it writes and applies those, so an
    ///   operation that writes none would print *no record* and the key would
    ///   do nothing. What most of them change is a surface's own state, and
    ///   this surface is the only thing holding it.
    /// - **The vocabulary does not name it at all.** `s` prints the status line
    ///   and `h`/`?` print [`BINDINGS`]. Neither has a row on
    ///   `docs/manual/operations.html`, which is the specification for which
    ///   **operations** exist — so neither is an operation anybody has
    ///   specified. Left as found and reported, because a row invented here
    ///   would be a specification written from the implementation.
    ///
    ///   *(This clause said that page is the specification for which keys
    ///   exist, and that stopped being true of **this** keyboard on
    ///   2026-08-29. Its `key` column is the instrument's keyboard — the keys
    ///   `cargo run -p karakuri` binds — and these thirty-nine are the command
    ///   line's own, which no column measures. The two collide on eight
    ///   letters. See
    ///   `docs/adr/0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md`.)*
    fn key(&mut self, key: &Key) -> bool {
        match key.as_ref() {
            Key::Named(NamedKey::Escape) => return true,
            Key::Named(NamedKey::Space) => self.toggle_focused(),
            Key::Character(s) => match s.chars().next().unwrap_or('\0').to_ascii_lowercase() {
                c @ '0'..='3' => self.focus_slot(c as usize - '0' as usize),
                '[' => self.nudge_gain(-GAIN_STEP),
                ']' => self.nudge_gain(GAIN_STEP),
                '\\' => self.set_gain(self.focus, 1.0),
                ';' => self.nudge_opacity(-OPACITY_STEP),
                '\'' => self.nudge_opacity(OPACITY_STEP),
                'm' => self.cycle_blend(self.focus),
                'f' => self.fade(0.0),
                'g' => self.fade(1.0),
                'x' => self.crossfade(),
                'c' => self.wipe(),
                'z' => self.cycle_mask(),
                'r' => self.cycle_renderer(),
                'n' => self.cycle_quantum(),
                'j' => self.cycle_fade_beats(),
                't' => self.cycle_tonemap(),
                '-' => self.set_exposure(self.look.exposure / EXPOSURE_STEP),
                '=' => self.set_exposure(self.look.exposure * EXPOSURE_STEP),
                '`' => self.set_exposure(1.0),
                'w' => self.toggle_priming(self.focus),
                'y' => self.cycle_sync(),
                'u' => self.scrub(-SCRUB_BEATS),
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
                's' => self.print_status(),
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
        eprintln!(
            "focus slot {slot} — {}, gain {:.2}, t {:.2}s",
            residency_name(self.deck.residency(slot), self.deck.is_parked(slot)),
            self.deck.gain(slot),
            self.deck.slot(slot).set().time()
        );
    }

    /// On air and off again. `Allocated` frees nothing and resets nothing, so
    /// the `t` printed on the way out is the `t` printed on the way back in —
    /// which is the whole property, and printing both ends is the only way to
    /// see it without a debugger.
    fn toggle_focused(&mut self) {
        self.toggle_on_air(self.focus);
    }

    /// On air and off again, for a named slot.
    ///
    /// **The toggle is the key's affordance and not an operation**, which is
    /// why the slot is a parameter and the focus is filled in by the caller:
    /// what reaches the deck is `SetResidency` naming one of three
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`). A control surface says which state it wants
    /// on the line and does not come through here at all — it used to, and the
    /// state it landed in was this function's to decide.
    fn toggle_on_air(&mut self, slot: usize) {
        let t = self.deck.slot(slot).set().time();
        match self.deck.residency(slot) {
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
    /// **A request, not a command** — the governor decides whether it is
    /// granted, and the report printed here says which. Whether it takes effect
    /// is exactly the thing the operator cannot otherwise see: a refused
    /// request leaves the slot at Allocated, which is what an untouched slot
    /// looks like too.
    ///
    /// Live slots are left alone. Priming is off-air warming, so asking a slot
    /// on air to prime could only mean taking it off air, and that is what
    /// space is for.
    fn toggle_priming(&mut self, slot: usize) {
        if self.deck.residency(slot) == Residency::Live {
            eprintln!("slot {slot} is on air — priming is off-air warming; take it off with space");
            return;
        }
        let requested = self.deck.requested_residency(slot);
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

    /// **Take up what a slot is now playing**, and say so in the stream if a
    /// session is being recorded.
    ///
    /// `landed` is the build id when a swap went in, or `None` when one was
    /// rolled back — which is the case [`Running::previous`] exists for. A
    /// rollback brings back a Set the stream will never name again, so the only
    /// way to say what came back is to have remembered it.
    ///
    /// **The bookkeeping is unconditional and the record is not**, and the two
    /// used to be one function that began by returning when there was no
    /// recorder. What a slot is playing was therefore a fact only a recorded run
    /// had — and `Live::save_set` needs exactly that fact in the ordinary
    /// `--watch` case, where nothing is being recorded. Splitting it is the
    /// whole of what widening `watch::Watch::stored` is for on this side of the
    /// channel.
    fn took_up(&mut self, slot: usize, landed: Option<u64>) {
        // Drained here rather than per frame: the channel only has anything in
        // it when a build has just been requested, and this runs when one has
        // just landed.
        if let Some(rx) = &self.rebuilds {
            while let Ok(built) = rx.try_recv() {
                self.pending_builds.insert(built.id, built);
            }
        }

        let pair = match landed {
            // **Missing means the watcher could not store this build's
            // sources**, which it said at the time. Handed to `landed` as
            // `None` rather than returned on, because the swap happened either
            // way: a slot that took a version nobody can name is a slot with no
            // address, not a slot still on its old one. See
            // [`Running::landed`], which is where that used to go wrong.
            Some(id) => {
                let built = self.pending_builds.remove(&id);
                self.running.landed(slot, built.map(|built| built.nodes))
            }
            None => self.running.rolled_back(slot),
        };
        let Some(nodes) = pair else {
            // A slot with nothing in the store behind it — one filled from a Set
            // file with nothing watching it, or one that has just taken a build
            // whose sources could not be stored, which was said at the time.
            // There is no hash to name, so there is nothing this could record.
            //
            // **A rollback to the launch version is no longer this case.** It
            // used to be, and the stream then said nothing at all about it, so a
            // replay went on drawing the build that had just been withdrawn.
            // The launch version is in `previous` from the first swap onward, so
            // it comes back like any other and is recorded like any other.
            return;
        };
        self.record_procedure(slot, &nodes);
    }

    /// Say what a slot is playing, now that it changed.
    ///
    /// **One thing this cannot carry.** The rollback restores the outgoing Set
    /// at the `t` it was parked at; a replay meeting these records builds
    /// afresh, so `t` restarts there. A swap *in* is documented to start cold
    /// and so replays exactly. Only a rollback differs, and a rollback means
    /// the candidate was over budget — an exceptional frame already.
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
                slot: slot as u8,
                layer: record,
                index: *index,
                proc_hash: *hash,
            });
        }
    }

    /// Push a record without applying it.
    ///
    /// **Two records are right here, and they are right for one reason**: each
    /// *describes* a change that has already happened rather than asking for
    /// one. A `procedure` says what a slot became when a swap landed, and a
    /// `save` says a file exists — applying either would mean doing the thing a
    /// second time. Everything else goes through `Live::record`, which applies
    /// what it wrote.
    fn record_only(&mut self, record: karakuri_store::record::Record) {
        if let Some(recorder) = &mut self.recorder {
            recorder.push(record);
        }
    }

    /// **Write what this run is playing as a Set file.**
    ///
    /// The control that closes an open gap: `--save-set`
    /// writes what the *flags* say and exits, so the loop that lets an operator
    /// load a preset, edit it and watch it had no way to keep the result. This
    /// is the render-loop half of closing that — see
    /// `docs/adr/0122-a-save-writes-the-bytes-that-are-on-screen.md`. Every surface ends here — the
    /// key below, the MCP tool, and whatever surface arrives next — for the
    /// same reason every mix control ends in one method. **This is the only save path**,
    /// which is what makes a refusal and an outcome one sentence each rather
    /// than one sentence per surface.
    ///
    /// **The slot is an argument and the key passes its focus in.** A Set file
    /// describes one Set and a deck holds four; the one an *operator* means is
    /// the one their hands are already on, which is what focus is — and a model
    /// has no hands and no focus, so it names the slot as it names one to read
    /// a procedure. This is `toggle_on_air`'s split, for its reason.
    ///
    /// **`id` is what the caller wanted it called, or a stamp.** A key press
    /// cannot type a name, so it passes `None`; see `history::stamped_id`,
    /// whose convention that is and whose reason it borrows — an operator looks
    /// for the time they saved it. A caller that *can* type one is not made to
    /// take a timestamp.
    ///
    /// **`reply` is whoever is waiting who is not at the terminal.** Every
    /// sentence below goes to both, and each of them is written once: a refusal
    /// that reached a model in different words than it reaches the terminal
    /// would be two refusals to keep in step, and the wording of this one has
    /// already had to be corrected once.
    ///
    /// **Read off the live Set, not off `Args`.** `saving_seeds` states the
    /// hazard from the other side: a writer with its own copy of the rule
    /// records numbers the run was not using. Every number a Set file carries
    /// can have moved since the flags were parsed — a param through a record, a
    /// capacity or a salt through a rebuilt Set file — so the only reading that
    /// cannot be stale is the Set's own.
    ///
    /// **Refused, accepted, gathered, written — and only the third of those
    /// needs a `Deck`.** The three sentences a save can produce before the disk
    /// speaks are [`no_such_slot`], [`nothing_to_save`] and [`accepted_save`],
    /// all free functions, so what a client is told is checkable without a
    /// window. `playing_values` is the line that is not, and everything above it
    /// here is above it deliberately.
    ///
    /// **Gathered here, written elsewhere.** Everything below this line is a
    /// read off values already in memory; the store I/O goes to a thread of its
    /// own — one per save, since saves are rare and a pool would be machinery
    /// for a rate of a few an hour. The outcome comes back over `saves` and the
    /// record is written at the frame it arrives, not at this key press. See
    /// `Live::finished_saves`.
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
        let id = accepted_save(slot, asked, id, &sources, &self.store_root, reply.as_ref());
        let values = playing_values(self.deck.slot(slot).set(), &self.edges);
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

    /// **Every save that has landed since the last frame, said and recorded.**
    ///
    /// Drained and never waited on: a frame owes the display a picture and owes
    /// a disk nothing.
    ///
    /// **Called at the top of the frame, beside [`Live::run_requests`] and
    /// above `frame::compose` and everything downstream of it** — not beside
    /// the swap-event drain, which is where it used to be, below the early
    /// returns a frame no longer has. See the comment at the head of [`Live::frame`]: a
    /// window that has faulted still has saves finishing behind it, and a run
    /// that told nobody about them until it quit was withholding the one answer
    /// a waiting client cannot get anywhere else. The swap drain stays below
    /// because a swap *is* about what was drawn; a save is not.
    ///
    /// **The record is written here, at the frame the outcome arrived**, which
    /// is the pattern `record_procedure` already follows — a record that
    /// describes a change already made. Writing one at the key press would be a
    /// stream claiming a file that the disk then refused, which is the failure
    /// this whole codebase is arranged against.
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
    /// **One sentence for all three.** What the terminal is told, what the
    /// stream records and what a waiting client is handed are the same fact, so
    /// the words are formed once here and the client gets the ones the operator
    /// got. The `Ok`/`Err` split is what a tool call's `isError` is built from —
    /// see [`mcp::Reply::settled`].
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
                    slot: slot as u8,
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

    /// **Every save still being written, waited for — up to [`SAVE_WAIT`].**
    ///
    /// A frame owes the disk nothing, which is why [`finished_saves`] drains and
    /// never blocks. The end of the run is the one moment where that is the
    /// wrong trade: a save pressed in the last second reached the disk under an
    /// id nothing in the stream ever named, so the claim that a `save`
    /// record exists for every live save that reached the disk
    /// (`docs/adr/0120-a-record-may-reach-outside-the-stream.md`)
    /// was false in
    /// exactly the window an operator is most likely to be in — press `k`, see
    /// it took, quit.
    ///
    /// **Bounded, because a disk can hang and quitting must not depend on one.**
    /// The alternative was joining the threads, which is unbounded by
    /// construction: a store on a network mount that stops answering would take
    /// the window with it. Past the bound the run says how many saves it left
    /// behind and exits, which is the same trade the recorder makes when it
    /// counts the batches it lost rather than waiting for them.
    ///
    /// The wait is only ever paid by a run that pressed `k` and quit within a
    /// few frames; the count is zero for every other run and this returns
    /// without blocking.
    fn awaited_saves(&mut self) {
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
    /// Called when something the decision depends on moved — a residency
    /// request, a build landing — and never per frame: it allocates, and the
    /// answer cannot change between those events.
    ///
    /// The parked slots are named individually rather than counted, because
    /// each one is waiting on something different and the reasons call for
    /// different actions: `NoHeadroom` waits for a slot to come off air,
    /// `CommittedUnknown` for a measurement, and `NoPrimingNeeded` for nothing
    /// at all — that Set is closed form and can go straight on air.
    fn govern(&mut self, why: &str) {
        report_governing(&self.deck.govern(), why);
    }

    /// **Cycle the focused slot's sync mode**, skipping the modes its material
    /// cannot take and saying why.
    ///
    /// This is the CLI's form of a control greyed out: there is no widget to
    /// dim, so the unavailable modes are stepped over and the reason is printed
    /// with the result. Silently skipping would leave an operator pressing a
    /// key and watching two of three modes never arrive; printing on every
    /// press without skipping would make the key refuse to do anything at all
    /// on material that only allows one mode.
    ///
    /// **Its record goes through [`Live::operate`] like every other settled
    /// key's**, and it used to be built here. What kept it out was
    /// `Owed::NotSettled` on [`Operation::SetSync`]: `Transport::engaged`
    /// clamps the anchor against the session tempo, so whether the record
    /// carried the tempo that was asked for or the one the engine settled on
    /// read as a decision about the bytes on disk. The two are the same number
    /// — the clamp holds the anchor inside
    /// `karakuri_signal::oscillator::BPM_RANGE` and an oscillator's tempo is
    /// already inside it — so what was missing was never a decision but a
    /// reading, and `Current::tempo` is it. A cycle is still this surface's
    /// own: `Sync::ALL`, the skipping and the refusals are translations a
    /// keyboard makes, and what comes out of them is a destination
    /// (P-0090).
    fn cycle_sync(&mut self) {
        let slot = self.focus;
        let current = self.deck.transport(slot).sync();
        let at = Sync::ALL.iter().position(|s| *s == current).unwrap_or(0);

        let mut refused: Vec<String> = Vec::new();
        let mut next = None;
        // Every other mode, in cycle order, starting after the current one.
        for step in 1..=Sync::ALL.len() {
            let candidate = Sync::ALL[(at + step) % Sync::ALL.len()];
            match self.deck.sync_allowed(slot, candidate) {
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
        let transport = self.deck.transport(slot);
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
            self.deck.transport(slot).scrub_beats()
        );
    }

    /// A tap on the beat. Authoritative — a performer tapping is stating where
    /// the beat is, not offering evidence — and it goes onto the oscillator
    /// through the same `tempo` record a tracked correction does.
    ///
    /// **Not through [`Live::operate`].** [`Operation::TapBeat`] is
    /// `Owed::NotSettled` — what a tap writes is the tracker's answer rather
    /// than a value, and the tracker may refuse — so this is also the one arm
    /// [`Live::run_surface`] keeps for itself. `b` and `note -> tap` are the
    /// same function for that reason, and both move the day the record is
    /// settled.
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

    /// Halve or double the grid — **the operator's last word on the octave**.
    ///
    /// The tracker folds every candidate tempo into a one-octave window centred
    /// on the grid, so an octave error is stable rather than self-correcting:
    /// a set started at 87 for a track that is 174 will track 87 all night. This
    /// moves the grid and the window together, and the picture keeps its phase
    /// — doubling subdivides the beats already there.
    ///
    /// **Not through [`Live::operate`]**, for [`Live::tap`]'s reason:
    /// [`Operation::ScaleGrid`] is `Owed::NotSettled` because moving the grid
    /// needs the beat tracker rather than a value, and the refusal below is the
    /// tracker's to give.
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
    /// measure. Found from where the audience stands, not from this machine —
    /// see `karakuri-audio`'s crate doc.
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
        self.set_gain(self.focus, self.deck.gain(self.focus) + delta);
    }

    fn set_gain(&mut self, slot: usize, gain: f32) {
        self.operate(&Operation::SetGain {
            deck: slot as u8,
            gain: clamp_gain(gain),
        });
        eprintln!("slot {slot} gain {:.2}", self.deck.gain(slot));
    }

    /// **The fader**, and the one control that silences a slot under every
    /// blend mode — which makes it the way out of material that has gone NaN.
    /// `[` and `]` move the *level*, and under `over` a level of zero is a
    /// black card that still covers what is beneath it.
    ///
    /// The mode is printed with the number because what the number does
    /// depends on it: under `add` opacity and gain are the same dial twice.
    fn nudge_opacity(&mut self, delta: f32) {
        let slot = self.focus;
        self.set_opacity(slot, self.deck.opacity(slot) + delta);
    }

    fn set_opacity(&mut self, slot: usize, value: f32) {
        let opacity = value.clamp(0.0, 1.0);
        self.operate(&Operation::SetOpacity {
            deck: slot as u8,
            opacity,
        });
        eprintln!(
            "slot {slot} opacity {:.2} ({})",
            self.deck.opacity(slot),
            self.deck.blend(slot).name()
        );
    }

    /// **Fade the focused slot's fader to `to`**, over the current length,
    /// starting on the current quantum.
    ///
    /// Opacity rather than gain, because opacity is the fader: it silences a
    /// slot under every blend mode, where a gain of zero under `over` is a
    /// black card that still covers. A gain fade is reachable through the
    /// record and deliberately has no key — two keys that look alike and differ
    /// only under one blend mode is how an operator ends up fading the wrong
    /// one in the dark. `Operation::FadeDeck` says the same thing at its own
    /// definition, which is why the choice is not repeated in the record this
    /// writes.
    ///
    /// **One `operate` call, and [`Live::fade_slot`] is gone.** This built its
    /// record where it stood while the quantum and the length had no owner;
    /// they are the surface's, they are handed over as
    /// `karakuri_operation_record::Current::transition`, and what this key
    /// writes is `written`'s answer like every other settled key's.
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

    /// **A crossfade: the focused slot out and the next one in, together.**
    ///
    /// Two scheduled moves rather than a `Crossfade` object, which is the whole
    /// argument of `karakuri_engine::transition` seen from the keyboard — the
    /// first-class thing is the move, and every gesture anyone names is made of
    /// those. They share a start and a length, so they are one gesture without
    /// being one type.
    ///
    /// **The incoming slot is put at silence and then on air**, in that order,
    /// and both halves of that are load-bearing. A slot comes up at full
    /// opacity and going off air does not lower it, so putting one on air
    /// without silencing it first shows it at full immediately — up to a bar
    /// before the fade it is supposed to arrive on, which is a cut with a
    /// decorative fade attached. And a fade to something that is not being
    /// composited is a fade to black, so it does have to go on air.
    ///
    /// The silencing is a `opacity` record like any other, so it cancels
    /// nothing the operator wanted and replays like anything else.
    ///
    /// **All four of its records are one operation now, and this function is
    /// one `operate` call** — which is the sentence that used to be here as a
    /// promise. It read *"the day that is settled this function is one
    /// `operate` call"*, and the day was the one the quantum and the length
    /// got an owner: they are the surface's,
    /// `karakuri_operation_record::Current::transition` is where this program
    /// hands them over, and `written` answers `Operation::Crossfade` with the
    /// four records in the order above.
    ///
    /// **The two remaining lines are the keyboard's translation and not the
    /// gesture.** *The next deck* is what `x` means here and the operation
    /// names both decks, so working out which one and refusing a deck with
    /// nowhere to go stays; everything past that is the conversion's.
    ///
    /// **The put-on-air is written even where the slot is already live**,
    /// which is the one thing this changed about the stream. It used to be
    /// conditional on `Deck::residency`, and the conversion has no deck to
    /// ask: `Operation::Crossfade` says four records at its own definition,
    /// and a `residency` record for a slot that is already live decodes to a
    /// state it is already in.
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

    /// **Wipe the next slot in over the focused one.**
    ///
    /// A mask and one scheduled move, and that is the whole of it: the incoming
    /// slot is given the current shape at position 0 — revealing nothing — put
    /// on air under `over` so that what it reveals *hides* what is beneath, and
    /// then one transition carries the front from 0 to 1. Nothing in the
    /// transition system knows what a mask is and nothing in the mask knows
    /// what a beat is.
    ///
    /// **All six of its records are one operation now, and this function is
    /// one `operate` call** — six where nothing is already where the wipe is
    /// putting it, and four or five where something is — which is
    /// [`Live::crossfade`]'s paragraph one gesture along and the last of them
    /// to be written. It built five of the
    /// six out of five separate operations and the sixth by hand, while what a
    /// wipe owed had no owner: the *shape* its front takes and the soft edge.
    /// Both have one. The shape is `Operation::SetTransition`'s third setting
    /// — the one this program holds in `mask_kind` and `mask_angle` and the
    /// `z` key writes — so it goes over with the quantum and the length inside
    /// `karakuri_operation_record::Current::transition`, which is where its
    /// two neighbours already were. The soft edge is read off the mask on the
    /// deck being wiped in, which is `mix::current_mask` and was already the
    /// reading `Operation::SetMaskShape` takes.
    ///
    /// **The two lines that are left are the keyboard's translation and not
    /// the gesture.** *The next slot* is what `c` means here and the operation
    /// names both decks, so working out which one and refusing a deck with
    /// nowhere to go stays. So does the refusal with no shape chosen:
    /// `Operation::Wipe` says *"Refused with no shape chosen"* at its own
    /// definition, `written` has no answer that is a refusal, and the shape is
    /// this surface's own setting — so this is the only place that can turn a
    /// wipe with nothing to move away, and it does it before it asks.
    ///
    /// **The one decision this function used to make is made in the
    /// conversion now, and it is not a record.** Under `add` the same gesture
    /// is a wipe *on* rather than a wipe *over*, which is a different picture
    /// and a legitimate one — so the mode is left wherever the operator had it
    /// and `over` is written only where the slot is still at the mode a slot
    /// starts in, which is what makes `m` in front of `c` mean something. The
    /// put-on-air is written only where the slot is not already live, on the
    /// same terms. Those were two `if`s here and they are the `Wipe` arm's
    /// now, because the sentence belongs beside the records it governs rather
    /// than on one of the surfaces that can reach them
    /// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// — a rule held in one surface binds none of the other three). What the
    /// conversion needed to keep it is a reading, which is the third this
    /// gesture takes: `karakuri_operation_record::Current::mix`, handed over
    /// by [`Live::operate`] out of the deck this function no longer touches.
    ///
    /// **Six records where it once wrote five**, and the extra one is what
    /// routing the mask honestly costs rather than an accident: the shape and
    /// the front are two operations
    /// (`docs/adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md`)
    /// and each of them writes a whole `Record::Mask`, because the record is a
    /// state and not an ask. The pair lands in that order, so the front is at
    /// 0 when the move is scheduled — which is the same picture the one
    /// hand-built record made.
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
    /// diagonals, and an iris. A continuous dial for the angle is not built:
    /// there is nowhere on this surface to see one.
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

    /// **Choose which renderer of the focused slot is the live one**, on the
    /// grid.
    ///
    /// The first half of a variant pool, and the half that is true today:
    /// several renderers over *one* simulation, in one Set, one of them folded
    /// into the picture at a time. A pool spanning deck slots that differ at a
    /// layer slot was the alternative and is rejected — see
    /// `docs/adr/0148-a-variant-pool-is-a-set-and-the-deck-stays-a-mixer.md` —
    /// and the rest of it — an alternative that
    /// differs at L1, priming it off air, sharing the geometry between them —
    /// is not built. See the manual, "Selecting one renderer of a slot".
    ///
    /// **What it does not save.** The renderers that are not selected go on
    /// drawing, each into a target of its own: at 1280x720 that is 7.03 MB and
    /// a render pass apiece, every frame, whether or not anybody is looking at
    /// them. That is what makes the selection a uniform write and a cut on the
    /// beat rather than a build — cheap, and not free. The line printed says
    /// how many are still drawing for exactly that reason.
    ///
    /// **Cycling from the selection that is armed, not from the one on
    /// screen.** A press lands up to a bar later, so two presses inside a bar
    /// read the same live edge and would both choose the same renderer — the
    /// second press would do nothing and say it had. `Deck::selections_on` is
    /// what makes the second press mean "the one after that".
    ///
    /// **One way, and it is stated rather than discovered**: there is no
    /// position in the cycle that puts every renderer back. A Set comes up with
    /// all of them folded — or folded to the one its Set file's `merge` record
    /// named, which is the other way to start — and the first press leaves that
    /// state for good, which is what "makes one live and the rest not" costs
    /// when the record names one renderer. Restoring the fold is a different
    /// statement and wants its own vocabulary — an `Option` carrying `null`
    /// for "all of them" is the shape it would take, which is the shape the
    /// retired `preview` record had.
    ///
    /// **What it chooses is kept.** A live save reads the fold off the Set —
    /// see `playing_values` — so `k` after a press writes `{"t":"merge",
    /// "live":N}` and loading that file back comes up on renderer N.
    ///
    /// **Its record comes out of [`Live::operate`]**, which it did not while
    /// the instant a selection lands on had nobody to supply it: the quantum
    /// is this program's setting, `mix::current_transition` is where it
    /// becomes an instant, and `written` writes the `select` record from it.
    /// Which renderer to move to is the keyboard's own translation and stays
    /// here, which is the whole of what is left of this function's arithmetic.
    fn cycle_renderer(&mut self) {
        let slot = self.focus;
        let set = self.deck.slot(slot).set();
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
        let armed = self.deck.selections_on(slot).next().map(|s| s.renderer());
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

    /// Cycle the focused slot's blend mode. No refusals here — unlike sync,
    /// every mode is available to every slot, because a blend mode is a
    /// question about pixels and not about what the material can do.
    fn cycle_blend(&mut self, slot: usize) {
        let current = self.deck.blend(slot);
        let at = Blend::ALL.iter().position(|b| *b == current).unwrap_or(0);
        let next = Blend::ALL[(at + 1) % Blend::ALL.len()];
        self.operate(&Operation::SetBlendMode {
            deck: slot as u8,
            blend: mix::blend_mode(next),
        });
        eprintln!(
            "slot {slot} blend {} — gain {:.2}, opacity {:.2}",
            self.deck.blend(slot).name(),
            self.deck.gain(slot),
            self.deck.opacity(slot)
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

    /// **Every mix change goes through here, and here goes through a record.**
    ///
    /// Built, decoded, and only then applied — so what drives the deck is what
    /// a replay would decode from a session stream, rather than a second path
    /// that happens to agree with it today. `karakuri-environment`'s `audio.rs`
    /// does the same thing with the two records it emits; see the program's
    /// `mix.rs` for the
    /// whole argument.
    ///
    /// A record this build cannot obey is printed and nothing moves. It cannot
    /// happen from a key press — every caller here built the record a moment
    /// ago out of the engine's own types — and it is handled rather than
    /// unwrapped because the replay driver will hand this same function lines
    /// off a file, and a file is where an unobeyable record comes from.
    /// A tempo correction into the stream.
    ///
    /// Separate from [`Live::record`] because a `tempo` is applied where it is
    /// decided rather than read back — the oscillator is moved by the code that
    /// worked out how far, and `apply_replayed` is what re-applies it on the
    /// way back. What this owes is the *writing*, and it is one function so
    /// that a third thing moving the grid cannot forget it: two already had.
    ///
    /// Scalars only, so pushing it allocates nothing, which is what lets the
    /// frame path call it as well as the two keys.
    fn push_tempo(&mut self, record: karakuri_store::record::Record) {
        if let Some(recorder) = &mut self.recorder {
            recorder.push(record);
        }
    }

    /// **A surface's operation, as the records it writes — and then written.**
    ///
    /// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// puts every control at the same record, and this is where a key
    /// press ends: the operation is named, `karakuri-operation-record` says
    /// what it writes, and [`Live::record`] writes it and reads it back the way
    /// every other record here is read back. A console fader and a mapped MIDI
    /// control are the same thing exactly because they arrive at this function
    /// carrying the same name.
    ///
    /// **The reading is taken here and nowhere else.** The conversion is not
    /// pure — `SetExposure` becomes a `look` record carrying the operator and
    /// the white point too — so what is running has to be read back and handed
    /// over, and this is the only place in this program that knows both what
    /// was asked for and what is on screen.
    ///
    /// The two answers that are not records are **printed rather than
    /// swallowed**. Nothing routed through here produces one today, which is
    /// exactly why silence would be the wrong response: reaching one means an
    /// operation was routed here that this build cannot write, and an operator
    /// pressing a key that does nothing deserves the sentence.
    fn operate(&mut self, operation: &Operation) {
        let transport = match operation {
            Operation::ScrubDeck { deck, .. } => {
                let slot = usize::from(*deck);
                slot_in_range(slot, self.deck.slot_count())
                    .then(|| mix::current_transport(self.deck.transport(slot)))
            }
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
            | Operation::Wipe { to: deck, .. } => {
                let slot = usize::from(*deck);
                slot_in_range(slot, self.deck.slot_count())
                    .then(|| mix::current_mask(self.deck.mask(slot)))
            }
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
            Operation::Wipe { to: deck, .. } => {
                let slot = usize::from(*deck);
                slot_in_range(slot, self.deck.slot_count())
                    .then(|| mix::current_mix(self.deck.blend(slot), self.deck.residency(slot)))
            }
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
            transport,
            mask,
            tempo,
            transition,
            mix,
        };
        match karakuri_operation_record::written(operation, &current) {
            Written::Records(records) => {
                for record in records {
                    self.record(record);
                }
            }
            Written::Silent(silent) => {
                eprintln!("{}: no record — {}", operation.title(), silent.why())
            }
            Written::Owed(owed) => eprintln!("{}: {}", operation.title(), owed.why()),
        }
    }

    fn record(&mut self, record: karakuri_store::record::Record) {
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
            mix::Change::Gain { slot, value } => self.deck.set_gain(slot, value),
            mix::Change::Opacity { slot, value } => self.deck.set_opacity(slot, value),
            mix::Change::Blend { slot, mode } => self.deck.set_blend(slot, mode),
            mix::Change::Mask { slot, mask } => self.deck.set_mask(slot, mask),
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
                let count = self.deck.slot(slot).set().inputs().len();
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
            mix::Change::Residency { slot, level } => {
                self.deck.set_residency(slot, level);
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
                if let Err(refusal) = self.deck.set_transport(slot, sync, anchor_bpm, scrub_beats) {
                    eprintln!("slot {slot}: {} sync refused — {refusal}", sync.name());
                }
            }
        }
    }

    fn frame(&mut self) {
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
        // measurement the deck is budgeting against — a swap and a rollback
        // both move what is committed. Collected here and governed after the
        // drain, because `Deck::events` borrows the deck for as long as it is
        // being read.
        let mut set_changed = false;
        // Collected rather than recorded inside the loop: `Deck::events`
        // borrows the deck for as long as it is read, and writing a record
        // needs the recorder.
        let mut procedures: Vec<(usize, Option<u64>)> = Vec::new();
        for slot in 0..self.deck.slot_count() {
            for event in self.deck.events(slot) {
                set_changed |= matches!(event, Event::Swapped { .. } | Event::RolledBack { .. });
                // **The same words, to whoever is not at the terminal.** A
                // model that wrote a procedure has no other way to learn that
                // it was rolled back for cost, and "it compiled" is not the
                // same news as "it is on screen".
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
                match event {
                    Event::Swapped { id, .. } => procedures.push((slot, Some(id))),
                    Event::RolledBack { .. } => procedures.push((slot, None)),
                    _ => {}
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

    /// What the operator needs and nothing that costs a stall to know: which
    /// slots are on air, what they are faded to, where their simulation clocks
    /// are, the output look, and whether frames are still arriving on time.
    ///
    /// Element live counts are deliberately absent — `Set::live_count` blocks
    /// until the queue drains, so a status line carrying it would put a GPU
    /// sync on the render thread twice a second. It is printed once, at exit.
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
            let _ = write!(
                self.status,
                "{}{slot} {} g{:.2} t{:.1}s ",
                if slot == self.focus { ">" } else { " " },
                residency_tag(self.deck.residency(slot), self.deck.is_parked(slot)),
                self.deck.gain(slot),
                self.deck.slot(slot).set().time()
            );
            // **What is moving, and where it is going.** An armed fade is
            // invisible otherwise: with the default quantum it is due up to a
            // bar after the key, and the only thing that said so was one line
            // at press time. A control that changes something invisible is
            // indistinguishable from a control that is broken, which is this
            // file's own argument for printing on every key.
            for t in self.deck.transitions_on(slot) {
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
            for selection in self.deck.selections_on(slot) {
                let _ = write!(self.status, "r>{} ", selection.renderer());
            }
            // The fader and the mode, **only when they are doing something**,
            // on the same terms as the transport below: a deck nobody has
            // touched prints the line it always printed. Full opacity under
            // `add` is what every slot comes up as, and a column repeating it
            // four times is four columns of nothing to read in the dark.
            if self.deck.opacity(slot) != 1.0 {
                let _ = write!(self.status, "o{:.2} ", self.deck.opacity(slot));
            }
            if self.deck.blend(slot) != Blend::Add {
                let _ = write!(self.status, "{} ", self.deck.blend(slot).name());
            }
            // The transport, and **only when it is doing something**: a deck
            // nobody has synced prints the line it always printed. `free` is
            // the absence of a transport rather than a setting, and a column
            // reading `free` on every slot would be four characters of nothing
            // on a line that has to be read at a glance in the dark.
            let transport = self.deck.transport(slot);
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
            match self.deck.level(slot) {
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
            for (key, value) in self.deck.slot(slot).set().bound() {
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

/// The status line's four-column form of the same thing [`residency_name`]
/// spells out. Fixed width, so the columns after it do not move.
///
/// Pulled out beside `residency_name` for the reason `slot_in_range` was: the
/// distinction it carries is the one thing about it that can be wrong, and
/// checking it should not need a GPU, a window, or a `Deck`.
fn residency_tag(residency: Residency, parked: bool) -> &'static str {
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
fn residency_name(residency: Residency, parked: bool) -> &'static str {
    match residency {
        Residency::Live => "live",
        Residency::Priming => "priming (warming, off air)",
        Residency::Allocated if parked => "parked (asked to prime, waiting for room)",
        Residency::Allocated => "allocated (off air)",
    }
}

/// Whether `slot` names one this deck actually has. Pulled out of
/// [`Live::focus_slot`] so the boundary — the neighbour of the off-by-one the
/// digit keys used to have, where a digit equal to or past the slot count
/// must be rejected rather than wrap or panic — is checkable without a
/// window, a GPU, or a `Deck`.
fn slot_in_range(slot: usize, slot_count: usize) -> bool {
    slot < slot_count
}

#[cfg(test)]
mod tests {
    use super::*;
    // **Reached from here and from nowhere above**, which is why they are not in
    // the file's own imports: the sort's `Err` arm and the tone map table are
    // both things only a test asks about directly.
    use karakuri_environment::compile::sort_compiled;
    use karakuri_environment::mix::{op_wire_name, TONEMAPS};
    use karakuri_signal::NoiseKind;

    fn parse(args: &[&str]) -> Result<Args, String> {
        match parse_args_from(args.iter().map(|s| s.to_string())) {
            Ok(ParseOutcome::Run(args)) => Ok(*args),
            Ok(ParseOutcome::Help) => panic!("expected Args, got --help"),
            Ok(ParseOutcome::ListSets(_)) => panic!("expected Args, got --list-sets"),
            Ok(ParseOutcome::Package { .. }) => panic!("expected Args, got --package"),
            Ok(ParseOutcome::TakeIn { .. }) => panic!("expected Args, got --take-in"),
            Err(e) => Err(e),
        }
    }

    // -- --list-sets -----------------------------------------------------

    /// **`--list-sets` is not a run, and the type says so.**
    ///
    /// The flag prints what the store holds and stops: no window, no adapter,
    /// no compile, no Set built. That is enforced by there being no [`Args`] at
    /// all on this path — [`parse_args_from`] answers with
    /// [`ParseOutcome::ListSets`], which carries a store path and nothing else,
    /// so everything `main` does with an `Args` is unreachable rather than
    /// merely skipped. A `bool` on `Args` would have needed a check above every
    /// early return in `main` and would have been wrong the day somebody added
    /// one more.
    ///
    /// **`--store` on either side of it**, because an operator types the flags
    /// in whatever order they think of them, and a listing of the default store
    /// when `--store` was given would be a listing of the wrong library.
    #[test]
    fn list_sets_prints_and_is_never_a_run() {
        for spelling in [
            vec!["--list-sets", "--store", "/tmp/library"],
            vec!["--store", "/tmp/library", "--list-sets"],
        ] {
            match parse_args_from(spelling.iter().map(|s| s.to_string())) {
                Ok(ParseOutcome::ListSets(root)) => {
                    assert_eq!(
                        root,
                        PathBuf::from("/tmp/library"),
                        "{spelling:?} listed a store nobody asked for"
                    );
                }
                Ok(ParseOutcome::Run(_)) => panic!(
                    "{spelling:?} came back as a run: a listing would then compile the \
                     default material and open a window to print a directory"
                ),
                Ok(ParseOutcome::Help) => panic!("{spelling:?} came back as --help"),
                Ok(ParseOutcome::Package { .. }) | Ok(ParseOutcome::TakeIn { .. }) => {
                    panic!("{spelling:?} came back as a transfer")
                }
                Err(e) => panic!("{spelling:?} was refused: {e}"),
            }
        }
    }

    /// **Neither `--package` nor `--take-in` is a run**, and the type says so
    /// for [`ParseOutcome::ListSets`]'s reason: there is no [`Args`] on either
    /// path, so the compile, the deck, the window and the adapter request are
    /// unreachable rather than merely skipped. `--take-in` does reach the
    /// checker — that is how a metadata card gets written — and the check pass
    /// needs no device.
    ///
    /// **`--store` on either side of each**, because an operator types the
    /// flags in whatever order they think of them, and a Set packaged out of the
    /// default store when `--store` was given would be a package of the wrong
    /// library.
    #[test]
    fn package_and_take_in_print_and_are_never_a_run() {
        for spelling in [
            vec!["--package", "night01", "--store", "/tmp/library"],
            vec!["--store", "/tmp/library", "--package", "night01"],
        ] {
            match parse_args_from(spelling.iter().map(|s| s.to_string())) {
                Ok(ParseOutcome::Package { store, id }) => {
                    assert_eq!(store, PathBuf::from("/tmp/library"), "{spelling:?}");
                    assert_eq!(id, "night01", "{spelling:?}");
                }
                Ok(ParseOutcome::Run(_)) => panic!(
                    "{spelling:?} came back as a run: writing a package to stdout would then \
                     compile material and open a window to do it"
                ),
                other => panic!(
                    "{spelling:?} came back as something else: {}",
                    named(&other)
                ),
            }
        }
        for spelling in [
            vec!["--take-in", "sent.kbset", "--store", "/tmp/library"],
            vec!["--store", "/tmp/library", "--take-in", "sent.kbset"],
        ] {
            match parse_args_from(spelling.iter().map(|s| s.to_string())) {
                Ok(ParseOutcome::TakeIn { store, file }) => {
                    assert_eq!(store, PathBuf::from("/tmp/library"), "{spelling:?}");
                    assert_eq!(file, PathBuf::from("sent.kbset"), "{spelling:?}");
                }
                Ok(ParseOutcome::Run(_)) => panic!(
                    "{spelling:?} came back as a run: taking a file into the store would \
                     then build a Set and open a window"
                ),
                other => panic!(
                    "{spelling:?} came back as something else: {}",
                    named(&other)
                ),
            }
        }
    }

    /// Which outcome, for a panic message. `ParseOutcome` holds GPU-adjacent
    /// config and is deliberately not `Debug`; see `value_tests::parse`.
    fn named(outcome: &Result<ParseOutcome, String>) -> String {
        match outcome {
            Ok(ParseOutcome::Run(_)) => "a run".to_string(),
            Ok(ParseOutcome::Help) => "--help".to_string(),
            Ok(ParseOutcome::ListSets(_)) => "--list-sets".to_string(),
            Ok(ParseOutcome::Package { .. }) => "--package".to_string(),
            Ok(ParseOutcome::TakeIn { .. }) => "--take-in".to_string(),
            Err(e) => format!("a refusal: {e}"),
        }
    }

    /// A store with two Sets in it, written at times this test decides.
    ///
    /// A Set file carries no time — the mtime is the only record of when one
    /// was saved — so a fixture that means to test an order has to say what the
    /// times are rather than hope two writes land in different seconds.
    fn library(root: &std::path::Path) -> karakuri_store::store::Store {
        let store = karakuri_store::store::Store::open(root).expect("store");
        let hash = store.put_artifact(b"not compiled here").expect("put");
        let slot = |layer: Layer, index: u32, name: &str| {
            karakuri_store::ndjson::Line::new(Record::Slot {
                layer,
                index,
                name: Some(name.to_string()),
                proc_hash: hash,
            })
        };
        store
            .write_set(
                "older",
                &[slot(Layer::L1, 0, "shell"), slot(Layer::L4, 0, "dots")],
            )
            .expect("set");
        store
            .write_set(
                "newer",
                &[
                    slot(Layer::L1, 0, "shell"),
                    slot(Layer::L2, 0, "bend"),
                    slot(Layer::L4, 0, "dots"),
                    slot(Layer::L4, 1, "strokes"),
                ],
            )
            .expect("set");
        for (id, secs) in [("older", 1_000u64), ("newer", 2_000)] {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .open(root.join("sets").join(format!("{id}.kbset")))
                .expect("open the set file");
            file.set_times(
                std::fs::FileTimes::new()
                    .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(secs)),
            )
            .expect("set the mtime");
        }
        store
    }

    /// **One line per Set: the id, when it was written, and what it holds.**
    ///
    /// The id is what goes next to `--load-set`, the time is what an operator
    /// looks for a keeper by, and the layers are enough to tell two Sets apart
    /// without opening either. Most recent first, for the reason the MCP
    /// listing is: *what did I just save* is the question.
    ///
    /// A layer nothing is on is left out rather than printed as a zero — most
    /// Sets are on three of the five, and a line of zeroes reads as something
    /// missing.
    #[test]
    fn the_listing_is_a_line_per_set_most_recent_first() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("store");
        let store = library(&root);
        let said = listed_sets(&store, &root).expect("the listing");

        let lines: Vec<&str> = said.lines().collect();
        assert_eq!(lines.len(), 2, "one line per set, and no more: {said}");
        assert!(
            lines[0].starts_with("newer") && lines[1].starts_with("older"),
            "the sets are not most recent first: {said}"
        );
        assert!(
            lines[0].contains("1 L1, 1 L2, 2 L4") && !lines[0].contains("L3"),
            "the line does not say what the set holds by layer, or counts a layer \
             nothing is on: {said}"
        );
        assert!(
            lines[1].contains("1 L1, 1 L4"),
            "the line does not say what the set holds by layer: {said}"
        );
        // The times are the ones the fixture set, in the operator's own clock,
        // and they are what tells two takes of one evening apart.
        for (line, secs) in lines.iter().zip([2_000u64, 1_000]) {
            let written = setfile::written_at(std::time::UNIX_EPOCH + Duration::from_secs(secs));
            assert!(
                line.contains(&written),
                "the line does not say when the set was written: {line}"
            );
        }
    }

    /// An empty store is an ordinary answer and says where sets come from — not
    /// a blank terminal, which reads as a flag that did nothing.
    #[test]
    fn an_empty_store_says_so_rather_than_printing_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("store");
        let store = karakuri_store::store::Store::open(&root).expect("store");
        let said = listed_sets(&store, &root).expect("the listing");
        assert!(
            said.contains("no sets in") && said.contains("--save-set"),
            "an empty store printed something an operator cannot act on: {said:?}"
        );

        // **And a path with no store at it is a different answer, and creates
        // nothing.** `Store::open` would establish the layout under it, so a
        // listing of a mistyped `--store` would answer "empty" and leave a
        // directory behind saying so — a read-only flag that writes.
        let missing = dir.path().join("nowhere");
        let said = listed_sets_at(&missing).expect("the listing");
        assert!(
            said.contains("no store at") && said.contains("--store"),
            "a path with no store at it was not told apart from an empty one: {said:?}"
        );
        assert!(
            !missing.exists(),
            "a listing created the store it was asked to read"
        );
    }

    /// **`--package` takes a `.kset` in and packages it**, which is the other
    /// moment of the one operation this flag already was — and the moment the
    /// flag is now named for.
    ///
    /// What is being checked here is the *route* and not the resolution — that
    /// is `karakuri-environment`'s, tested there — so this asserts the two
    /// things only this file decides: that a value ending in `.kset` is read as
    /// a path to an authoring file rather than looked up as an id, and that
    /// what comes back is a bundle, sources and all, on the standard output a
    /// shell can redirect.
    ///
    /// **And that the store it writes into is established**, unlike the id
    /// half: resolving is a write, so a `--store` an operator named has to
    /// exist by the time the first artifact lands.
    #[test]
    fn package_takes_an_authoring_file_in_and_packages_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let l1 = "proc ring { kind L1 }\n";
        let l4 = "proc points { kind L4 }\n";
        std::fs::write(dir.path().join("l1.kir"), l1).expect("write");
        std::fs::write(dir.path().join("l4.kir"), l4).expect("write");
        let kset = dir.path().join("night.kset");
        std::fs::write(
            &kset,
            "{\"t\":\"set\",\"id\":\"night\",\"v\":1}\n\
             {\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n\
             {\"t\":\"part\",\"layer\":\"L4\",\"path\":\"l4.kir\"}\n",
        )
        .expect("write");

        let root = dir.path().join("store");
        let said = packaged_set(&root, kset.to_str().expect("utf-8")).expect("the packaging");

        assert!(
            !said.contains("\"t\":\"part\""),
            "a bundle carries no part: {said}"
        );
        assert_eq!(
            said.matches("\"t\":\"slot\"").count(),
            2,
            "each part became a slot naming an address: {said}"
        );
        for source in [l1, l4] {
            let hash = karakuri_store::hash::Hash::of(source.as_bytes());
            assert!(
                said.contains(&hash.to_string()),
                "the slot names the address of the bytes on disk: {said}"
            );
            assert!(
                karakuri_store::store::Store::open(&root)
                    .expect("store")
                    .get_artifact(&hash)
                    .is_ok(),
                "the store this wrote into holds the part: {said}"
            );
        }
        assert!(
            said.contains("\"t\":\"src\""),
            "the sources are inlined, which is what makes it a bundle: {said}"
        );

        // **And an id is still an id.** The extension is the whole of what
        // tells the two apart, so a value without one is looked up in the store
        // and says so when it is not there.
        let refused = packaged_set(&root, "night").expect_err("no set is filed under that id");
        assert!(refused.contains("night"), "{refused}");
    }

    /// **`--take-in` takes both of a Set's forms**, and the extension is what
    /// says which — the same sentence [`packaged_set`] reads, from the other
    /// end.
    ///
    /// A `.kbset` is already resolved and is taken in as it stands; that is what
    /// `karakuri-environment`'s own tests cover. What only this file decides is
    /// the `.kset` half: a value ending in `.kset` is resolved against its own
    /// directory first and then taken in, so an operator who was sent an
    /// authoring file beside its parts does not have to package it to themselves
    /// before they can keep it.
    ///
    /// **And it lands the same store as packaging it would**, which is why the
    /// route is `bundle_authored` and not `resolve` alone: the artifacts are
    /// there, the Set is filed under the id the file carries, and each source
    /// has the metadata card a take-in writes.
    #[test]
    fn take_in_resolves_an_authoring_file_and_then_takes_it_in() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Sources that actually compile, because a take-in runs the checker to
        // write a metadata card and a source it will not compile is stored
        // without one.
        let l1 = r#"
proc ring {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  emit position

  element {
    position = vec3(cos(t), sin(t), 0.0);
  }
}
"#;
        let l4 = r#"
proc points {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.016;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
        std::fs::write(dir.path().join("l1.kir"), l1).expect("write");
        std::fs::write(dir.path().join("l4.kir"), l4).expect("write");
        let kset = dir.path().join("night.kset");
        std::fs::write(
            &kset,
            "{\"t\":\"set\",\"id\":\"night\",\"v\":1}\n\
             {\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n\
             {\"t\":\"part\",\"layer\":\"L4\",\"path\":\"l4.kir\"}\n",
        )
        .expect("write");

        let root = dir.path().join("store");
        let said = taken_in_file(&root, &kset).expect("the take-in");
        assert!(
            said.contains("`night`") && said.contains("2 sources stored"),
            "the report does not say what was taken in: {said}"
        );
        assert!(
            said.contains("2 metadata cards written"),
            "a take-in writes a card per source, whichever form it came in: {said}"
        );

        let store = karakuri_store::store::Store::open(&root).expect("store");
        assert!(
            store
                .list_sets()
                .expect("sets")
                .iter()
                .any(|e| e.id == "night"),
            "the set is not filed under the id the file carries"
        );
        for source in [l1, l4] {
            let hash = karakuri_store::hash::Hash::of(source.as_bytes());
            assert!(
                store.get_artifact(&hash).is_ok(),
                "the store does not hold the part the authoring file named"
            );
        }

        // **And the id in the file is still somebody else's word.** Taking the
        // same authoring file in twice is the refusal a `.kbset` gets, for the
        // same reason: nothing here was typed by the operator.
        let refused = taken_in_file(&root, &kset).expect_err("the id is taken");
        assert!(
            refused.contains("already in this store"),
            "a second take-in overwrote a Set: {refused}"
        );
    }

    // -- edges -----------------------------------------------------------

    /// **`--edge <node>.<slot>=<geometry>` writes the record it stands for**,
    /// and every part of the spelling names something.
    ///
    /// The `.` is the same dot the procedure reads the slot through and the `=`
    /// is `--set`'s, so `--edge morph.far=sphere` and `far.position` in the
    /// `deform` are visibly one spelling.
    #[test]
    fn an_edge_names_a_node_a_slot_and_the_geometry_bound_to_it() {
        let args = parse(&["--edge", "morph.far=sphere_shell"]).expect("parses");
        assert_eq!(
            args.edges,
            vec![karakuri_engine::set::Edge {
                node: "morph".to_string(),
                slot: "far".to_string(),
                to: "sphere_shell".to_string(),
            }]
        );

        // **The last dot, not the first.** A node name may hold one — nothing
        // refuses `--set my.morph=morph.kir` — and a slot is a `.kir`
        // identifier, which cannot.
        let args = parse(&["--edge", "my.morph.far=sphere"]).expect("parses");
        assert_eq!(args.edges[0].node, "my.morph");
        assert_eq!(args.edges[0].slot, "far");
    }

    /// **A malformed edge is refused rather than dropped**, on `--param`'s
    /// terms: an edge that was silently discarded looks exactly like a slot
    /// nobody bound, and the refusal for *that* would name the file rather than
    /// the command line that misspelt it.
    #[test]
    fn a_malformed_edge_says_so() {
        for spelled in [
            // No geometry after the `=`.
            "morph.far",
            // No slot: a sentence about a node, which there is no such thing as.
            "morph=sphere",
            // Every part names something.
            "morph.=sphere",
            ".far=sphere",
            "morph.far=",
        ] {
            let err = parse(&["--edge", spelled])
                .expect_err(&format!("`{spelled}` is not an edge"))
                .to_string();
            assert!(err.contains("--edge"), "`{spelled}` -> {err}");
        }
    }

    // -- audio -----------------------------------------------------------

    #[test]
    fn audio_is_off_unless_asked_for_and_takes_a_device_name() {
        assert_eq!(parse(&[]).expect("parses").audio_in, None);
        assert_eq!(
            parse(&["--audio-in", "default"]).expect("parses").audio_in,
            Some("default".to_string())
        );
        assert_eq!(
            parse(&["--audio-in", "Scarlett"]).expect("parses").audio_in,
            Some("Scarlett".to_string())
        );
        // A flag with nothing after it takes the next flag as its value in the
        // shape this CLI was fixed for once already.
        assert!(parse(&["--audio-in", "--watch"]).is_err());
        assert!(parse(&["--audio-in"]).is_err());
    }

    /// **A saved Set records what the run was drawing**, which is the one
    /// promise the format makes and the one it was breaking.
    ///
    /// `--save-set` wrote `args.capacity` — the flag's number, or its default
    /// where no flag was given — while the run asks each procedure for the
    /// default *it* declares. A lattice written for 32768 elements was saved as
    /// 262144 and loaded back a larger, smeared version of itself.
    ///
    /// It was invisible while `--load-set` also ignored what the file said: the
    /// number was wrong on the way out and wrong again on the way in, and the
    /// two cancelled. Teaching the loader to honour a recorded capacity is what
    /// made them disagree out loud, which is the ordinary way a pair of
    /// compensating errors is found.
    #[test]
    fn a_saved_set_records_the_capacity_the_run_was_drawing() {
        let small = compile::check(
            "proc small { kind L1 capacity [4096, 262144] = 32768 topology points \
             emit position element { position = vec3(0.0); } }",
        )
        .expect("compiles");
        let large = compile::check(
            "proc large { kind L1 capacity [4096, 1048576] = 131072 topology points \
             emit position element { position = vec3(0.0); } }",
        )
        .expect("compiles");

        let args = parse(&[]).expect("parses");
        assert_eq!(
            saving_capacities(&args, &[small.clone(), large.clone()]),
            vec![32768, 131072],
            "the file has to say what each geometry drew, not what the flag defaults to"
        );

        // And `--capacity` is still the operator overriding every source, so
        // that is what a file saved under it records.
        let forced = parse(&["--capacity", "8192"]).expect("parses");
        assert_eq!(
            saving_capacities(&forced, &[small, large]),
            vec![8192, 8192]
        );
    }

    /// **Each source runs at the capacity its own procedure declares.** The
    /// build asked `capacity_for` once, about the first L1, and handed the
    /// answer to every source — so a grid declared at 131072 ran at 32768
    /// because it was loaded beside a cube that declared that.
    ///
    /// Nothing could have caught it downstream: the number came from a real
    /// declaration, so it was inside *somebody's* range, and the picture is a
    /// grid with fewer points in it, which is a thing a grid can be.
    #[test]
    fn every_source_runs_at_the_capacity_it_declares() {
        let small = compile::check(
            "proc small { kind L1 capacity [4096, 262144] = 32768 topology points \
             emit position element { position = vec3(0.0); } }",
        )
        .expect("compiles");
        let large = compile::check(
            "proc large { kind L1 capacity [4096, 1048576] = 131072 topology points \
             emit position element { position = vec3(0.0); } }",
        )
        .expect("compiles");

        let args = parse(&[]).expect("parses");
        assert_eq!(
            capacities_for(&args, &[small.clone(), large.clone()], &[]),
            vec![32768, 131072]
        );
        // Order is not what decides it, which is the half a first-one-wins
        // implementation gets right by accident half the time.
        assert_eq!(
            capacities_for(&args, &[large.clone(), small.clone()], &[]),
            vec![131072, 32768]
        );

        // `--capacity` still overrides all of them: the operator asking for a
        // number is asking about the Set, not about one file in it.
        let forced = parse(&["--capacity", "8192"]).expect("parses");
        assert_eq!(
            capacities_for(&forced, &[small.clone(), large.clone()], &[]),
            vec![8192, 8192]
        );

        // **And a Set file's own number wins over both**, per geometry: the
        // file is where that geometry's count was decided, and a Set that came
        // back at another size is a Set that was not saved. Where the file said
        // nothing, the declaration underneath still answers.
        assert_eq!(
            capacities_for(&args, &[small, large], &[Some(16384), None]),
            vec![16384, 131072]
        );
    }

    // -- midi ------------------------------------------------------------

    #[test]
    fn midi_is_off_unless_asked_for_and_takes_a_port_name() {
        assert_eq!(parse(&[]).expect("parses").midi_in, None);
        assert_eq!(parse(&[]).expect("parses").midi_map, None);
        assert_eq!(
            parse(&["--midi-in", "nanoKONTROL"])
                .expect("parses")
                .midi_in,
            Some("nanoKONTROL".to_string())
        );
        // An empty selector is "the first port there is", which is a real
        // answer rather than a missing value — the one plugged-in surface is
        // the common case and should need no name.
        assert_eq!(
            parse(&["--midi-in", ""]).expect("parses").midi_in,
            Some(String::new())
        );
        assert!(parse(&["--midi-in", "--watch"]).is_err());
        assert!(parse(&["--midi-in"]).is_err());
        // With a port, so what refuses this is `--midi-map` swallowing the next
        // flag rather than the map-with-no-port rule getting there first. That
        // is the difference between this line and an assertion that cannot
        // fail.
        assert!(parse(&["--midi-in", "", "--midi-map", "--watch"]).is_err());
        assert!(parse(&["--midi-in", "", "--midi-map"]).is_err());
    }

    /// **A map with no port is a file nothing reads**, and the likely cause is
    /// a forgotten `--midi-in`. Refused where it can be said rather than
    /// loaded, checked and silently unused.
    #[test]
    fn a_midi_map_with_no_port_is_refused() {
        let message = parse(&["--midi-map", "surface.map"]).expect_err("a map with nothing to map");
        assert!(message.contains("--midi-in"), "{message}");
        assert!(parse(&["--midi-map", "surface.map", "--midi-in", ""]).is_ok());
    }

    /// An offscreen run has nobody at the surface, on exactly the terms it has
    /// no microphone.
    #[test]
    fn midi_and_an_offscreen_render_are_refused_together() {
        let message =
            parse(&["--midi-in", "", "--render", "out.png"]).expect_err("should be refused");
        assert!(message.contains("--midi-in"), "{message}");
        assert!(parse(&["--midi-in", "", "--seq", "frames/"]).is_err());
        assert!(parse(&["--midi-in", ""]).is_ok());
    }

    /// An offscreen run is a function of its arguments, so a live input is
    /// refused rather than accepted and ignored.
    #[test]
    fn audio_and_an_offscreen_render_are_refused_together() {
        let message = parse(&["--audio-in", "default", "--render", "out.png"])
            .expect_err("should be refused");
        assert!(message.contains("--audio-in"), "{message}");
        assert!(parse(&["--audio-in", "default", "--seq", "frames/"]).is_err());
        // ...and either alone is fine.
        assert!(parse(&["--audio-in", "default"]).is_ok());
        assert!(parse(&["--render", "out.png"]).is_ok());
    }

    #[test]
    fn the_latency_offset_defaults_takes_a_sign_and_is_validated() {
        assert_eq!(
            parse(&[]).expect("parses").latency_offset_ms,
            audio::DEFAULT_LATENCY_OFFSET_MS
        );
        assert_eq!(
            parse(&["--latency-offset-ms", "35"])
                .expect("parses")
                .latency_offset_ms,
            35.0
        );
        // **Negative is a value, not a typo.** A room whose sound arrives after
        // its picture is corrected for by turning this below zero, and there is
        // no other control that can: nothing at this end sees the PA or the
        // projector.
        assert_eq!(
            parse(&["--latency-offset-ms", "-45"])
                .expect("a negative offset is a room, not a mistake")
                .latency_offset_ms,
            -45.0
        );
        // Refused rather than clamped or defaulted: an offset silently changed
        // is an offset the operator spends the first song chasing.
        for bad in ["1000", "-1000", "twenty", ""] {
            assert!(
                parse(&["--latency-offset-ms", bad]).is_err(),
                "`--latency-offset-ms {bad}` was accepted"
            );
        }
    }

    // -- defaults and the positional pair --------------------------------

    /// **A session recorded with no flags carries its material and replays.**
    ///
    /// The head used to be the `--load-set` file or nothing, and "or nothing"
    /// meant a timeline of ticks with no material under it: `--replay` refused
    /// it with "has no L1 slot" long after the set was over, and nothing said
    /// so at the time. This is the round trip that catches it — the head is
    /// written the way the recorder writes it and read back the way the replay
    /// reads it, so a head that describes nothing fails here rather than on
    /// stage.
    #[test]
    fn a_session_head_carries_the_material_a_replay_needs() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let l1 = root.join("examples/drift_shell.kir");
        let l4 = root.join("examples/soft_points.kir");
        let dir = tempfile::tempdir().expect("tempdir");
        let store = karakuri_store::store::Store::open(dir.path()).expect("store");

        let mut args = parse(&[]).expect("parses");
        args.sets = vec![(Named::bare(&l1), vec![Named::bare(&l4)])];
        // **And the edge that makes it buildable.** `morph` takes a geometry it
        // calls `far` and a Set that does not say which one is refused where it
        // is built — so a head recorded without it is a head no replay can
        // open, which is exactly the failure this test is about.
        args.edges = vec![karakuri_engine::set::Edge {
            node: "morph".to_string(),
            slot: "far".to_string(),
            to: "sphere_shell".to_string(),
        }];
        args.store = dir.path().to_path_buf();
        let (material, placed) = sort_slot(0, &args.sets[0].0, &args.sets[0].1);

        let head = session_head(&args, &[placed], &material.l1s, &store, "a_set");
        assert!(!head.is_empty(), "the head describes nothing");

        // Read back the way `--replay` reads it, which is the whole claim: the
        // artifacts resolve out of the store and both slots are there.
        let loaded = setfile::from_lines(&store, "a_set", &head)
            .expect("the head a recording writes is a head a replay can load");
        assert_eq!(loaded.l1s[0].kind, karakuri_ir::Kind::L1);
        assert_eq!(loaded.l4s[0].kind, karakuri_ir::Kind::L4);
        assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
    }

    /// **A session opens with a Set file, so what a Set file cannot hold is a
    /// performance that cannot be recorded.**
    ///
    /// A cube morphing into a sphere is two geometries, an L2 that pairs them
    /// and a renderer — a chain `--set` has spelled for a while and the head
    /// could not carry: `session_head` wrote every path after the first as an
    /// `L4` slot, so `--record-session` refused it outright rather than
    /// recording a performance nobody could replay. This is that chain through
    /// the head and back, layer by layer.
    #[test]
    fn a_session_head_carries_a_whole_chain_and_not_just_a_pair() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dir = tempfile::tempdir().expect("tempdir");
        let store = karakuri_store::store::Store::open(dir.path()).expect("store");

        let mut args = parse(&[]).expect("parses");
        args.sets = vec![(
            Named::bare(root.join("examples/lattice_shell.kir")),
            vec![
                Named::bare(root.join("examples/sphere_shell.kir")),
                Named::bare(root.join("examples/morph.kir")),
                Named::bare(root.join("examples/soft_points.kir")),
            ],
        )];
        args.store = dir.path().to_path_buf();
        let (material, placed) = sort_slot(0, &args.sets[0].0, &args.sets[0].1);

        let head = session_head(&args, &[placed], &material.l1s, &store, "a_chain");
        let loaded = setfile::from_lines(&store, "a_chain", &head)
            .expect("the head a recording writes is a head a replay can load");
        assert_eq!(
            loaded
                .l1s
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            ["lattice_shell", "sphere_shell"],
            "the geometries, in the order they were named"
        );
        assert_eq!(
            loaded
                .l2s
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            ["morph"],
            "the deformer that pairs them"
        );
        assert_eq!(
            loaded
                .l4s
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            ["soft_points"]
        );
        assert_eq!(loaded.edges, args.edges, "and the wiring between them");
        assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
    }

    /// One slot's examples, compiled, as [`sort_compiled`] takes them — with
    /// the text each was compiled from, which is what a node is addressed by.
    fn compiled(files: &[&str]) -> Vec<(Named, karakuri_ir::typed::Checked, std::sync::Arc<str>)> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        files
            .iter()
            .map(|file| {
                let path = root.join(file);
                let (checked, src) = compile::load(&path).expect("the examples compile");
                (
                    Named::bare(path),
                    checked,
                    std::sync::Arc::from(src.as_str()),
                )
            })
            .collect()
    }

    /// **A slot that cannot be assembled refuses with a sentence**, and the two
    /// callers decide what to do about it — a startup prints it and stops, a
    /// rebuild prints it and leaves the Set that is running alone. That
    /// difference is the only thing the two paths do differently, and it is the
    /// reason this sort was written twice before it was written once.
    ///
    /// The sentence names the file, because the file is what an operator can
    /// fix; the slot is prefixed by whichever caller is reporting it.
    #[test]
    fn a_slot_refuses_nothing_that_draws_and_keeps_a_second_camera_and_field() {
        // Matched rather than `expect_err`, which would want `Material` to be
        // `Debug` — a derive on a production type to print something no test
        // reaching here ever prints.
        let refused = |files: &[&str]| match sort_compiled(compiled(files)) {
            Err(e) => e,
            Ok(_) => panic!("this slot cannot be assembled"),
        };
        // **A second camera is not a refusal, and this is where that stopped
        // being one.** It said a slot looks from one viewpoint, which was true
        // of the plumbing and not of the material: a renderer declares `uses
        // view : Camera` and an `edge` names which node fills it, so two
        // cameras are two nodes addressed as `L3:0` and `L3:1`.
        let (two_cameras, placed_cameras) = match sort_compiled(compiled(&[
            "drift_shell.kir",
            "beat_jump.kir",
            "beat_jump.kir",
            "soft_points.kir",
        ])) {
            Ok(sorted) => sorted,
            Err(e) => panic!("two cameras are two nodes: {e}"),
        };
        assert_eq!(two_cameras.l3s.len(), 2, "both were kept");
        assert_eq!(
            placed_cameras
                .iter()
                .filter(|p| p.layer == karakuri_ir::Kind::L3)
                .map(|p| (p.layer, p.index))
                .collect::<Vec<_>>(),
            [(karakuri_ir::Kind::L3, 0), (karakuri_ir::Kind::L3, 1)],
            "the cameras are numbered from 0 with no gaps"
        );
        // **A second field is not a refusal, and this is where that stopped
        // being one.** The sort was the last thing in the tree saying a Set
        // holds one, and it said so about the plumbing rather than about the
        // material: two fields are two nodes, addressed as `Field:0` and
        // `Field:1` and bound by name.
        let two_fields = sort_compiled(compiled(&[
            "drift_shell.kir",
            "melt_blob.kir",
            "melt_blob.kir",
            "soft_points.kir",
        ]));
        let (material, placed) = match two_fields {
            Ok(sorted) => sorted,
            Err(e) => panic!("two fields are two nodes: {e}"),
        };
        assert_eq!(material.fields.len(), 2, "both were kept");
        // **At its own index**, which is what `--param Field:1:x` and a Set
        // file's `slot` record both address it by. The second used to be
        // dropped, and before the refusal above it was dropped silently.
        let addresses: Vec<(karakuri_ir::Kind, u32)> = placed
            .iter()
            .filter(|p| p.layer == karakuri_ir::Kind::Field)
            .map(|p| (p.layer, p.index))
            .collect();
        assert_eq!(
            addresses,
            [(karakuri_ir::Kind::Field, 0), (karakuri_ir::Kind::Field, 1)],
            "the fields are numbered from 0 with no gaps"
        );
        let no_renderer = refused(&["drift_shell.kir", "swirl_warp.kir"]);
        assert!(no_renderer.contains("nothing here draws"), "{no_renderer}");
        // The file the slot was spelled with, which is the one an operator looks
        // at first — and by the time this is said the list has been sorted past.
        assert!(no_renderer.contains("drift_shell.kir"), "{no_renderer}");

        // ...and one of each is a slot, so none of the above is a refusal of
        // cameras and fields as such.
        let one_of_each = sort_compiled(compiled(&[
            "drift_shell.kir",
            "beat_jump.kir",
            "melt_blob.kir",
            "soft_points.kir",
        ]));
        assert!(
            one_of_each.is_ok(),
            "one camera and one field is a slot, not a refusal"
        );
    }

    /// **Any part of a `--set` may carry a name.** A name addresses the node
    /// from every surface that can reach one, and it is written where the file
    /// is spelled because it belongs to the *use* — the same lattice twice is
    /// one `proc` name and two nodes.
    #[test]
    fn a_set_may_name_any_of_its_files() {
        let args = parse(&["--set", "near=a.kir,far=b.kir,c.kir"]).expect("parses");
        assert_eq!(
            args.sets,
            vec![(
                Named {
                    name: Some("near".to_string()),
                    path: PathBuf::from("a.kir")
                },
                vec![
                    Named {
                        name: Some("far".to_string()),
                        path: PathBuf::from("b.kir")
                    },
                    Named::bare("c.kir"),
                ]
            )]
        );
    }

    /// **Two written names that collide are refused rather than resolved**, and
    /// this is the *early* refusal — the one that names the slot, before a GPU
    /// is asked for anything. `Set::build_many` refuses the same thing again
    /// where every node is in hand, which is where a derived name could also
    /// collide with a written one.
    #[test]
    fn two_nodes_cannot_share_a_name() {
        let names = Names {
            l1s: vec![Some("near".to_string())],
            l4s: vec![Some("near".to_string())],
            ..Names::default()
        };
        let e = names.check_unique().expect_err("refused");
        assert!(e.contains("both called `near`"), "{e}");

        // And the scope is the slot, because a Set is what holds the nodes.
        let fine = Names {
            l1s: vec![Some("near".to_string())],
            l4s: vec![Some("draw".to_string())],
            ..Names::default()
        };
        assert!(fine.check_unique().is_ok());
    }

    /// **A layer spelling cannot be a node's name**, because `--param` tells the
    /// two forms apart by what follows the first colon. A node called `L4` would
    /// make `--param L4:0:x` ambiguous with a node named `L4` holding a param
    /// called `0`.
    #[test]
    fn a_node_name_is_refused_where_it_would_be_read_as_something_else() {
        for bad in ["L1", "L4", "Field"] {
            let e = parse(&["--set", &format!("{bad}=a.kir,b.kir")]).expect_err("refused");
            assert!(e.contains("is a layer"), "{e}");
        }
        // A name is an address, so what may be in one is narrow and the
        // refusal names the character rather than quietly rewriting it: a
        // silently renamed node is one a `--param` written against it stops
        // finding.
        let e = parse(&["--set", "a b=a.kir,b.kir"]).expect_err("refused");
        assert!(e.contains("not allowed"), "{e}");
        let e = parse(&["--set", "=a.kir,b.kir"]).expect_err("refused");
        assert!(e.contains("empty"), "{e}");
        let e = parse(&["--set", "near=,b.kir"]).expect_err("refused");
        assert!(e.contains("no file after it"), "{e}");
    }

    #[test]
    fn no_arguments_is_the_default_pair() {
        let args = parse(&[]).expect("should parse");
        assert_eq!(
            args.sets,
            vec![(
                Named::bare("examples/drift_shell.kir"),
                vec![Named::bare("examples/soft_points.kir")],
            )]
        );
    }

    /// **A demonstration brings the scene it is about.** `--demo lines` fades
    /// each slot out in turn so the other is seen alone, and on a one-slot deck
    /// that shows the picture and then an empty frame — the demonstration would
    /// run, look like it worked, and demonstrate nothing.
    ///
    /// **Two slots, therefore, even though a stack would fit in one.** This was
    /// briefly rewritten to a single slot holding both renderers, on the
    /// grounds that it is the shape the milestone made possible — which broke
    /// it, because the script takes one slot at a time out of the mix and a
    /// fader is per slot rather than per renderer. Slot 0 carries the stack,
    /// which is what the milestone actually buys here: the same two draws, over
    /// one simulation instead of two.
    #[test]
    fn the_lines_demo_supplies_its_own_two_slot_deck() {
        let args = parse(&["--demo", "lines"]).expect("should parse");
        assert_eq!(
            args.sets,
            vec![
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
            "there has to be more than one slot for taking one away to show anything"
        );
        // **Three moments, one per slot plus the return to the mix**, and each
        // moment is the keys it takes: a digit to focus the slot and `f` to
        // fade it out, `g` to bring the previous one back. A deck of a
        // different size leaves the script out of phase, which is what this
        // counts.
        let fades = DEMO_LINES_SCRIPT.iter().filter(|(_, k)| *k == 'f').count();
        let restores = DEMO_LINES_SCRIPT.iter().filter(|(_, k)| *k == 'g').count();
        assert_eq!(
            (fades, restores),
            (args.sets.len(), args.sets.len()),
            "the script fades one slot out per slot in the deck and brings each back; \
             a deck of a different size leaves it out of phase"
        );
        let focused: Vec<char> = DEMO_LINES_SCRIPT
            .iter()
            .filter(|(_, k)| k.is_ascii_digit())
            .map(|(_, k)| *k)
            .collect();
        assert_eq!(
            focused,
            vec!['1', '0'],
            "a fade acts on the focused slot, so every `f` needs the digit that says which"
        );
    }

    /// And supplies it rather than imposing it: material named on the command
    /// line still wins, so `--demo lines` over someone else's pair shows their
    /// material and not the examples.
    #[test]
    fn material_on_the_command_line_beats_a_demos_own_deck() {
        let args = parse(&["--demo", "lines", "a.kir", "b.kir"]).expect("should parse");
        assert_eq!(
            args.sets,
            vec![(Named::bare("a.kir"), vec![Named::bare("b.kir")])]
        );
    }

    /// The transport demonstration works on whatever is loaded, so it brings
    /// nothing and the ordinary default still applies. The control for the two
    /// above: a `deck()` that answered the same for every demonstration would
    /// satisfy them and break this.
    #[test]
    fn the_transport_demo_leaves_the_default_pair_alone() {
        let args = parse(&["--demo", "transport"]).expect("should parse");
        assert_eq!(
            args.sets,
            vec![(
                Named::bare("examples/drift_shell.kir"),
                vec![Named::bare("examples/soft_points.kir")],
            )]
        );
    }

    /// Both scripts must end after their last press, or the loop restarts
    /// mid-gesture and what a watcher sees depends on when they looked.
    #[test]
    fn every_demo_script_finishes_before_it_loops() {
        for demo in [Demo::Transport, Demo::Lines] {
            let last = demo.script().last().expect("a script with entries").0;
            assert!(
                demo.loop_seconds() > last,
                "{demo:?} loops at {}s, before its last press at {last}s",
                demo.loop_seconds()
            );
        }
    }

    #[test]
    fn a_bare_positional_pair_still_works() {
        let args = parse(&["a.kir", "b.kir"]).expect("should parse");
        assert_eq!(
            args.sets,
            vec![(Named::bare("a.kir"), vec![Named::bare("b.kir")])]
        );
    }

    #[test]
    fn one_positional_file_is_rejected() {
        let err = parse(&["a.kir"]).unwrap_err();
        assert!(err.contains("1 file argument"), "message: {err}");
    }

    #[test]
    fn three_positional_files_are_rejected() {
        let err = parse(&["a.kir", "b.kir", "c.kir"]).unwrap_err();
        assert!(err.contains("3 file argument"), "message: {err}");
    }

    #[test]
    fn a_positional_pair_becomes_the_last_slot_after_set() {
        let args = parse(&["--set", "a.kir,b.kir", "c.kir", "d.kir"]).expect("should parse");
        assert_eq!(
            args.sets,
            vec![
                (Named::bare("a.kir"), vec![Named::bare("b.kir")]),
                (Named::bare("c.kir"), vec![Named::bare("d.kir")]),
            ]
        );
    }

    // -- --set, adversarially ---------------------------------------------

    #[test]
    fn set_with_no_value_fails() {
        let err = parse(&["--set"]).unwrap_err();
        assert!(err.contains("--set"), "message: {err}");
    }

    #[test]
    fn set_with_one_path_and_no_comma_fails() {
        let err = parse(&["--set", "a.kir"]).unwrap_err();
        assert!(err.contains("--set a.kir"), "message: {err}");
    }

    /// **A bare `--param` is a wildcard and stays one.** Every node declaring
    /// the name — "the Set's `exposure`", one knob moving both renderers —
    /// which is what this flag has always meant, so the address arriving must
    /// not quietly turn it into "node 0".
    #[test]
    fn a_param_with_no_address_reaches_every_node_declaring_it() {
        let args = parse(&["--param", "exposure=2.5"]).expect("should parse");
        assert_eq!(
            args.overrides,
            vec![ParamWrite::everywhere("exposure", 2.5)],
            "a bare name must stay unaddressed"
        );
    }

    /// And the prefix is what sets two renderers apart, which a bare name
    /// cannot do by construction.
    #[test]
    fn a_param_can_address_one_renderer() {
        let args = parse(&["--param", "L4:1:exposure=2.5", "--param", "L1:0:radius=3.0"])
            .expect("should parse");
        assert_eq!(
            args.overrides,
            vec![
                ParamWrite::at(karakuri_ir::Kind::L4, 1, "exposure", 2.5),
                ParamWrite::at(karakuri_ir::Kind::L1, 0, "radius", 3.0),
            ]
        );
    }

    /// A half-address is refused rather than read as a name with a colon in it.
    /// The address is `layer:index:` present or absent as a unit, and a param
    /// name cannot contain a colon, so there is nothing else `L4:exposure` can
    /// be trying to say.
    #[test]
    fn a_half_written_param_address_fails() {
        for bad in [
            "L4:exposure=2.5",
            "L4:x:exposure=2.5",
            "L9:0:exposure=2.5",
            ":0:e=1",
            "=2.5",
        ] {
            let err = parse(&["--param", bad]).unwrap_err();
            assert!(
                err.contains("--param"),
                "`{bad}` was accepted or misreported: {err}"
            );
        }
    }

    /// `--bind` needed no new grammar at all: its fields are the record's, so
    /// the address is one more field. Absent is a wildcard there too.
    #[test]
    fn a_bind_can_address_one_renderer_and_defaults_to_all_of_them() {
        let all = parse(&["--bind", "layer=L4,key=exposure,signal=beat,range=0..1"])
            .expect("should parse");
        assert_eq!(
            all.bindings[0].index, None,
            "a bind with no index is the layer's"
        );

        let one = parse(&[
            "--bind",
            "layer=L4,index=2,key=exposure,signal=beat,range=0..1",
        ])
        .expect("should parse");
        assert_eq!(one.bindings[0].index, Some(2));

        let err = parse(&["--bind", "layer=L4,index=x,key=e,signal=beat,range=0..1"]).unwrap_err();
        assert!(err.contains("index"), "a bad index is not named: {err}");
    }

    /// **A third path is a second renderer**, and there is no new syntax for it.
    ///
    /// This used to be an error, and the reasoning was sound while a Set was a
    /// pair: `b.kir,c.kir` had been silently accepted as one literal L4
    /// filename, so a stray comma got blamed on a missing file. Now a Set holds
    /// a list, and one comma-separated list read as one L1 and however many L4s
    /// is exactly what the command line should look like — *no new syntax at
    /// all*. See
    /// `docs/adr/0154-a-third-path-on-set-is-a-second-renderer-and-there-is-no-new-syntax.md`.
    #[test]
    fn set_with_three_paths_is_one_geometry_and_two_renderers() {
        let args = parse(&["--set", "a.kir,b.kir,c.kir"]).expect("should parse");
        assert_eq!(
            args.sets,
            vec![(
                Named::bare("a.kir"),
                vec![Named::bare("b.kir"), Named::bare("c.kir")],
            )],
            "the first path is the geometry and the rest are renderers, in draw order"
        );
    }

    /// The stray comma the rule above used to catch is still caught, because an
    /// empty part is not a filename: `a.kir,b.kir,` names a renderer with no
    /// name.
    #[test]
    fn set_with_a_trailing_comma_fails() {
        let err = parse(&["--set", "a.kir,b.kir,"]).unwrap_err();
        assert!(err.contains("--set"), "message: {err}");
    }

    #[test]
    fn set_with_an_empty_value_fails() {
        let err = parse(&["--set", ""]).unwrap_err();
        assert!(err.contains("--set"), "message: {err}");
    }

    #[test]
    fn set_with_an_empty_side_fails() {
        assert!(parse(&["--set", ",b.kir"]).is_err());
        assert!(parse(&["--set", "a.kir,"]).is_err());
    }

    #[test]
    fn set_given_five_times_exceeds_max_slots() {
        let err = parse(&[
            "--set", "a,b", "--set", "c,d", "--set", "e,f", "--set", "g,h", "--set", "i,j",
        ])
        .unwrap_err();
        assert!(err.contains("5 Sets"), "message: {err}");
    }

    #[test]
    fn a_bare_pair_alongside_four_sets_exceeds_max_slots() {
        let err = parse(&[
            "--set", "a,b", "--set", "c,d", "--set", "e,f", "--set", "g,h", "i.kir", "j.kir",
        ])
        .unwrap_err();
        assert!(err.contains("5 Sets"), "message: {err}");
    }

    #[test]
    fn set_given_exactly_four_times_is_allowed() {
        let args = parse(&[
            "--set", "a,b", "--set", "c,d", "--set", "e,f", "--set", "g,h",
        ])
        .expect("4 sets should fit MAX_SLOTS");
        assert_eq!(args.sets.len(), 4);
    }

    // -- --exposure, adversarially -----------------------------------------

    #[test]
    fn exposure_missing_value_fails() {
        assert!(parse(&["--exposure"]).is_err());
    }

    #[test]
    fn exposure_non_number_fails() {
        let err = parse(&["--exposure", "bright"]).unwrap_err();
        assert!(err.contains("--exposure bright"), "message: {err}");
    }

    #[test]
    fn exposure_negative_fails() {
        assert!(parse(&["--exposure", "-2"]).is_err());
    }

    #[test]
    fn exposure_zero_fails() {
        assert!(parse(&["--exposure", "0"]).is_err());
    }

    #[test]
    fn exposure_positive_is_accepted_unclamped() {
        // Parsing does not clamp — only the interactive control does, on
        // purpose, so a batch render can ask for something extreme. `100`
        // is well outside the interactive `-`/`=` bound.
        let args = parse(&["--exposure", "100"]).expect("should parse");
        assert_eq!(args.look.exposure, 100.0);
    }

    // -- --tonemap, adversarially -------------------------------------------

    #[test]
    fn tonemap_bad_name_fails() {
        let err = parse(&["--tonemap", "bloom"]).unwrap_err();
        assert!(err.contains("--tonemap bloom"), "message: {err}");
    }

    #[test]
    fn tonemap_every_documented_name_is_accepted() {
        for (name, op) in [
            ("clamp", TonemapOp::Clamp),
            ("reinhard", TonemapOp::Reinhard),
            ("aces", TonemapOp::Aces),
            ("agx", TonemapOp::AgX),
        ] {
            let args = parse(&["--tonemap", name]).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(args.look.op, op, "{name}");
        }
    }

    // -- --bind ---------------------------------------------------------------

    /// The spec's own example, field for field. This is the mapping the flag
    /// exists to preserve: when a Set file can be loaded, each `field=value`
    /// here becomes the JSON field of the same name and nothing else changes.
    ///
    /// ```ndjson
    /// {"t":"bind","layer":"L1","key":"spawn_rate","signal":"noise",
    ///  "noise":{"kind":"perlin","rate":0.5,"stream":3},"curve":"lin","range":[4000,16000]}
    /// ```
    #[test]
    fn bind_carries_every_field_of_the_record() {
        let args = parse(&[
            "--bind",
            "layer=L1,key=spawn_rate,signal=noise,curve=lin,range=4000..16000,\
             noise.kind=perlin,noise.rate=0.5,noise.stream=3",
        ])
        .expect("should parse");
        assert_eq!(args.bindings.len(), 1);
        let b = &args.bindings[0];
        assert_eq!(b.layer, karakuri_ir::Kind::L1);
        assert_eq!(b.key, "spawn_rate");
        assert_eq!(b.signal, "noise");
        assert_eq!(b.curve, Curve::Lin);
        assert_eq!(b.range, [4000.0, 16000.0]);
        assert_eq!(
            b.noise,
            Some(NoiseConfig {
                kind: NoiseKind::Perlin,
                rate: 0.5,
                stream: 3,
            })
        );
    }

    #[test]
    fn bind_defaults_the_curve_and_the_noise_generator_but_nothing_else() {
        let plain =
            parse(&["--bind", "layer=L4,key=hue,signal=beat,range=0..1"]).expect("should parse");
        assert_eq!(plain.bindings[0].curve, Curve::Lin);
        assert_eq!(
            plain.bindings[0].noise, None,
            "only a noise signal gets one"
        );

        let noise = parse(&["--bind", "layer=L1,key=spawn_rate,signal=noise,range=0..1"])
            .expect("should parse");
        assert_eq!(
            noise.bindings[0].noise,
            Some(NoiseConfig::default()),
            "a noise signal with no generator named is the default generator, not none"
        );
    }

    #[test]
    fn bind_octaves_reaches_fbm_in_either_order() {
        for fields in [
            "layer=L1,key=radius,signal=noise,range=0..1,noise.kind=fbm,noise.octaves=6",
            "layer=L1,key=radius,signal=noise,range=0..1,noise.octaves=6,noise.kind=fbm",
        ] {
            let args = parse(&["--bind", fields]).expect("should parse");
            assert_eq!(
                args.bindings[0].noise.expect("a generator").kind,
                NoiseKind::Fbm { octaves: 6 },
                "{fields}"
            );
        }
        // `fbm` with nothing said about octaves is the record's default rather
        // than a rejection — the spec's own example omits it.
        let args = parse(&[
            "--bind",
            "layer=L1,key=radius,signal=noise,range=0..1,noise.kind=fbm",
        ])
        .expect("should parse");
        assert_eq!(
            args.bindings[0].noise.expect("a generator").kind,
            NoiseKind::Fbm {
                octaves: setfile::DEFAULT_OCTAVES
            }
        );
    }

    /// `octaves` belongs to `fbm` and, per `docs/ir-spec.md`, is "ignored by
    /// the other three kinds". What it must never do is decide the kind: a
    /// `noise.kind=white` that came back as `fbm` is a different generator
    /// from the one the operator named, chosen silently, in a flag whose whole
    /// stated reason for refusing unknown fields is that silence.
    #[test]
    fn bind_octaves_never_silently_replaces_the_kind_that_was_named() {
        for fields in [
            "layer=L1,key=r,signal=noise,range=0..1,noise.kind=white,noise.octaves=6",
            "layer=L1,key=r,signal=noise,range=0..1,noise.octaves=6,noise.kind=white",
            "layer=L1,key=r,signal=noise,range=0..1,noise.kind=perlin,noise.octaves=6",
            // No kind at all: `perlin` is the default, and an octave count is
            // as meaningless against it as against an explicit one.
            "layer=L1,key=r,signal=noise,range=0..1,noise.octaves=6",
        ] {
            let err = match parse(&["--bind", fields]) {
                Ok(args) => panic!(
                    "`{fields}` was accepted as {:?}",
                    args.bindings[0].noise.expect("a generator").kind
                ),
                Err(e) => e,
            };
            assert!(err.contains("noise.octaves"), "{fields} -> {err}");
        }
    }

    /// Every way of getting it wrong says which part was wrong. A `--bind`
    /// that quietly took a default would be a parameter that does not move and
    /// no way to find out why — the silence every other flag here was fixed
    /// for.
    #[test]
    fn a_malformed_bind_is_refused_and_says_what_it_could_not_use() {
        for (fields, expect) in [
            ("key=hue,signal=beat,range=0..1", "no `layer=`"),
            ("layer=L4,signal=beat,range=0..1", "no `key=`"),
            ("layer=L4,key=hue,range=0..1", "no `signal=`"),
            ("layer=L4,key=hue,signal=beat", "no `range="),
            (
                "layer=L4,key=hue,signal=beat,curve=expo,range=0..1",
                "expected lin, pow2, sqrt, smooth",
            ),
            ("layer=L4,key=hue,signal=beat,range=0-1", "LOW..HIGH"),
            ("layer=L4,key=hue,signal=beat,range=low..high", "LOW..HIGH"),
            (
                "layer=L4,key=hue,signal=beat,curv=lin,range=0..1",
                "unknown field `curv`",
            ),
            (
                "layer=L4,key=hue,signal=beat,range=0..1,noise.rate=2",
                "needs `signal=noise`",
            ),
            (
                "layer=L1,key=r,signal=noise,range=0..1,noise.rate=fast",
                "cycles per beat",
            ),
            ("layer=L4,key=hue,beat", "is not `field=value`"),
        ] {
            let err = match parse(&["--bind", fields]) {
                Ok(_) => panic!("`{fields}` was accepted"),
                Err(e) => e,
            };
            assert!(err.contains(expect), "{fields} -> {err}");
        }
    }

    /// A binding to `bpm` is pinned at the top of its range for the whole run,
    /// because a tempo is not a `[0, 1]` signal and the curve clamps it. That
    /// is `--param key=HIGH` spelled at four times the length, and nothing on
    /// the outside distinguishes it from a binding that is working — the
    /// status line shows a number, and the number never moves. `bpm` is a
    /// signal the spec lists, so a generator will reach for it; the refusal is
    /// what tells it to reach for `beat` instead.
    #[test]
    fn binding_bpm_is_refused_and_names_the_signal_to_use_instead() {
        let err = parse(&["--bind", "layer=L1,key=radius,signal=bpm,range=1..5"]).unwrap_err();
        assert!(err.contains("bpm"), "{err}");
        assert!(
            err.contains("beat"),
            "the refusal does not say what to use: {err}"
        );

        // The two that do carry the tempo in the range a binding needs are
        // still accepted, or the refusal above would just be a ban on tempo.
        for signal in ["beat", "bar"] {
            parse(&[
                "--bind",
                &format!("layer=L1,key=radius,signal={signal},range=1..5"),
            ])
            .unwrap_or_else(|e| panic!("`{signal}` was refused: {e}"));
        }
    }

    // -- --bpm ----------------------------------------------------------------

    #[test]
    fn bpm_defaults_and_refuses_a_tempo_it_cannot_use() {
        assert_eq!(parse(&[]).expect("should parse").bpm, DEFAULT_BPM);
        assert_eq!(parse(&["--bpm", "128"]).expect("should parse").bpm, 128.0);
        for bad in ["0", "-4", "fast"] {
            let err = parse(&["--bpm", bad]).unwrap_err();
            assert!(err.contains("--bpm"), "{bad} -> {err}");
        }
    }

    // -- unknown options ------------------------------------------------------

    #[test]
    fn an_unknown_dash_option_fails_rather_than_becoming_a_path() {
        let err = parse(&["--wtach"]).unwrap_err();
        assert!(err.contains("unknown option"), "message: {err}");
    }

    // -- seeds --------------------------------------------------------------

    #[test]
    fn slot_zero_keeps_the_original_seed() {
        assert_eq!(seed_for(0), SEED);
    }

    /// **What a Set file recorded is what the run uses, and an ordinal fills in
    /// the rest.**
    ///
    /// The two halves are one function on purpose: this is what a slot is built
    /// with *and* what `--save-set` writes down, so a file cannot record a salt
    /// the run was not using — the failure [`saving_capacities`] was fixed
    /// after, one field along. An unsaved `--set` is the ordinals, which is
    /// what makes saving a no-op on the picture and reloading a reproduction of
    /// it.
    #[test]
    fn a_recorded_salt_wins_and_an_unrecorded_one_is_derived() {
        let seed = seed_for(0);
        let derived = |at| karakuri_engine::set::derived_salt(seed, at);

        // A bare `--set a.kir,b.kir`: nothing recorded anything, so both are
        // the ordinals — and source 0's is the Set's seed unchanged, which is
        // what keeps a one-geometry run the run it always was.
        assert_eq!(salts_for(seed, &[], 2), vec![derived(0), derived(1)]);
        assert_eq!(salts_for(seed, &[], 1), vec![seed]);

        // A Set file that recorded both. Neither is an ordinal, and neither
        // moves when the geometries change places — which is the whole point.
        assert_eq!(salts_for(seed, &[Some(11), Some(22)], 2), vec![11, 22]);

        // And one that recorded fewer salts than the Set has geometries: an
        // older file, where a `seed` salted the Set rather than a source. What
        // it named keeps its colours and the rest are derived, which is what
        // one number could ever have meant.
        assert_eq!(salts_for(seed, &[Some(11)], 2), vec![11, derived(1)]);
    }

    /// **The flag and the file agree about compositing in either order**, and
    /// leaving the flag off takes nothing away from a file that records a
    /// merge.
    ///
    /// `--merge` can only turn compositing *on* — there is no spelling that
    /// turns it off, because overdraw is what saying nothing means — so
    /// `--load-set X --merge 0` on a composited file is two ways of asking for
    /// one thing rather than a contest. The case that decides the rule is the
    /// third: a composited file loaded with **no** flag. Reading the missing
    /// flag as "overdraw" would make every saved variant pool come back
    /// unfoldable, which is exactly the state the `merge` record was added to
    /// end.
    #[test]
    fn the_flag_and_the_file_agree_about_compositing_in_either_order() {
        use karakuri_engine::set::Layering::{Composite, Overdraw};
        let composited = FromSet {
            layering: Composite,
            ..FromSet::default()
        };
        let flagged = parse(&["a.kir", "b.kir", "--merge", "0"]).expect("args");
        let bare = parse(&["a.kir", "b.kir"]).expect("args");
        // Parsed again rather than cloned: `Args` is not `Clone`, and it is the
        // same two spellings either way.
        let loaded_args = || parse(&["a.kir", "b.kir"]).expect("args");
        let flagged_args = || parse(&["a.kir", "b.kir", "--merge", "0"]).expect("args");

        // The flag alone, which is what every composited slot was before a Set
        // file could say so.
        assert_eq!(
            layering_for(&flagged, 0, recorded_layering(&flagged, 0)),
            Composite
        );
        // The file alone: `--load-set` of a Set that was saved compositing.
        let mut loaded = loaded_args();
        loaded.from_set = Some(composited.clone());
        assert_eq!(
            layering_for(&loaded, 0, recorded_layering(&loaded, 0)),
            Composite,
            "a composited Set file loaded back overdrawing because no --merge was typed"
        );
        // Both, in one run.
        let mut both = flagged_args();
        both.from_set = Some(composited);
        assert_eq!(
            layering_for(&both, 0, recorded_layering(&both, 0)),
            Composite
        );
        // Neither.
        assert_eq!(
            layering_for(&bare, 0, recorded_layering(&bare, 0)),
            Overdraw
        );
        // **And the file is slot 0's.** `--load-set` fills that slot and every
        // other comes from `--set`, so a composited file says nothing about
        // slot 1 — where only the flag can.
        assert_eq!(
            layering_for(&loaded, 1, recorded_layering(&loaded, 1)),
            Overdraw
        );
    }

    /// **What a live save writes as the merge's `live`, read off the Set.**
    ///
    /// `k` and the MCP tool write what is on screen, and the fold is one of the
    /// things only the Set knows: there is no flag that selects a renderer.
    /// Every-input-live is `None` and not renderer 0 — a Set nobody has
    /// selected in has made no choice to record, and a one-renderer Set is that
    /// case rather than a selection of its only renderer.
    #[test]
    fn a_saved_fold_is_the_selection_the_set_is_holding() {
        let live = karakuri_engine::mix::Input::unity();
        let dark = karakuri_engine::mix::Input {
            live: false,
            ..karakuri_engine::mix::Input::unity()
        };
        // Nobody has selected: every input live, so there is nothing to write.
        assert_eq!(selected_renderer(&[live, live, live]), None);
        // One renderer, never selected in — "all of them" and "one of them" at
        // once, and it is the first.
        assert_eq!(selected_renderer(&[live]), None);
        // A selection, which is the one shape `mix::select` leaves.
        assert_eq!(selected_renderer(&[dark, live, dark]), Some(1));
        assert_eq!(selected_renderer(&[live, dark]), Some(0));
        // Shapes nothing can produce: recorded as unselected rather than as a
        // guess at which of them was meant.
        assert_eq!(selected_renderer(&[dark, dark]), None);
        assert_eq!(selected_renderer(&[live, live, dark]), None);
    }

    #[test]
    fn every_slot_gets_a_distinct_seed() {
        let seeds: Vec<u32> = (0..MAX_SLOTS).map(seed_for).collect();
        for i in 0..seeds.len() {
            for j in (i + 1)..seeds.len() {
                assert_ne!(seeds[i], seeds[j], "slots {i} and {j} collide");
            }
        }
    }

    // -- key-handling logic, separated from winit and the GPU ---------------

    #[test]
    fn slot_bounds_reject_the_slot_count_itself_and_beyond() {
        // The neighbour of the fixed off-by-one: digits are indices, so the
        // valid range is `0..slot_count`, and `slot_count` itself is already
        // out of range.
        assert!(slot_in_range(0, 4));
        assert!(slot_in_range(3, 4));
        assert!(!slot_in_range(4, 4));
        assert!(!slot_in_range(9, 4));
        // A deck of one: only slot 0 is valid, matching the default run.
        assert!(slot_in_range(0, 1));
        assert!(!slot_in_range(1, 1));
    }

    /// A parked slot is `Residency::Allocated`, exactly like a slot nobody
    /// asked about, and the operator's request is the only thing that tells
    /// them apart. Showing one as the other is not a cosmetic loss: it says a
    /// standing request was discarded, when the governor is reconsidering it
    /// every pass and will grant it the moment a slot comes off air.
    #[test]
    fn a_parked_slot_does_not_read_as_one_nobody_asked_about() {
        assert_ne!(
            residency_tag(Residency::Allocated, true),
            residency_tag(Residency::Allocated, false)
        );
        assert_ne!(
            residency_name(Residency::Allocated, true),
            residency_name(Residency::Allocated, false)
        );
        // Parked is off air, so it must not read as either of the two states
        // that are not: a park shown as `prim` claims warming that is not
        // happening, and shown as `LIVE` claims a slot on air.
        for parked in [true, false] {
            assert_ne!(
                residency_tag(Residency::Allocated, parked),
                residency_tag(Residency::Priming, parked)
            );
            assert_ne!(
                residency_tag(Residency::Allocated, parked),
                residency_tag(Residency::Live, parked)
            );
        }
        // `parked` is only ever true of an Allocated slot — `Deck::is_parked`
        // says so — but the tag is fixed-width regardless of what it is asked,
        // because the columns after it are positional.
        for residency in [Residency::Live, Residency::Priming, Residency::Allocated] {
            for parked in [true, false] {
                assert_eq!(residency_tag(residency, parked).len(), 4);
            }
        }
    }

    #[test]
    fn gain_floors_at_zero_but_has_no_ceiling() {
        assert_eq!(clamp_gain(-5.0), 0.0);
        assert_eq!(clamp_gain(0.0), 0.0);
        assert_eq!(clamp_gain(1.0), 1.0);
        assert_eq!(clamp_gain(1000.0), 1000.0, "HDR gain is not capped at 1.0");
    }

    #[test]
    fn exposure_clamps_into_a_finite_positive_range() {
        assert_eq!(clamp_exposure(0.0), EXPOSURE_MIN);
        assert_eq!(clamp_exposure(-5.0), EXPOSURE_MIN);
        assert_eq!(clamp_exposure(1_000_000.0), EXPOSURE_MAX);
        assert_eq!(clamp_exposure(1.0), 1.0);
    }

    /// The cycle visits every operator and closes — and, because `next_tonemap`
    /// is an exhaustive match while `TONEMAPS` is a hand-written list, **this
    /// is also what checks the list is complete**. An operator added to the
    /// enum forces a new arm in the cycle; if `TONEMAPS` is not updated with it
    /// the two disagree here, before it can reach a `look` record that spells a
    /// name nothing parses.
    #[test]
    fn tonemap_cycles_through_all_four_and_back_to_the_start() {
        let start = TonemapOp::Clamp;
        let mut op = start;
        let mut seen = vec![op];
        for _ in 0..TONEMAPS.len() - 1 {
            op = next_tonemap(op);
            seen.push(op);
        }
        assert_eq!(
            seen,
            vec![
                TonemapOp::Clamp,
                TonemapOp::Reinhard,
                TonemapOp::Aces,
                TonemapOp::AgX,
            ]
        );
        assert_eq!(next_tonemap(op), start, "the cycle must close");
        assert_eq!(
            seen.len(),
            TONEMAPS.len(),
            "the cycle and `TONEMAPS` disagree about how many operators there are"
        );
        for op in TONEMAPS {
            assert!(seen.contains(&op), "{} is not in the cycle", op_name(op));
        }
    }

    /// Both spellings of every operator are distinct from every other's, so a
    /// `look` record cannot name two operators and `--tonemap` cannot resolve
    /// to the wrong one. Two arms of `spellings` sharing a wire name would
    /// compile and would make `parse_op` return whichever came first.
    #[test]
    fn no_two_tonemap_operators_share_a_spelling() {
        for op in TONEMAPS {
            assert_eq!(
                parse_op(op_wire_name(op)),
                Some(op),
                "`{}` does not parse back to itself",
                op_wire_name(op)
            );
        }
        let mut wire: Vec<&str> = TONEMAPS.iter().map(|op| op_wire_name(*op)).collect();
        wire.sort_unstable();
        let before = wire.len();
        wire.dedup();
        assert_eq!(before, wire.len(), "two operators share a wire spelling");
    }

    /// **A `save` after the last tick is named, not counted.**
    ///
    /// The `ir-spec` rule is that a replay says which effects outside the stream
    /// it skipped, and a save at the end of a set is the one that lands after
    /// the last tick rather than inside a frame. Paired with its control — a
    /// trailing record that is *not* a save gets the count and nothing more, so
    /// the assertion is about the `save` arm rather than about a function that
    /// prints an extra line whatever it is given.
    #[test]
    fn a_trailing_save_is_named_and_not_only_counted() {
        let notes = trailing_notes(&[
            Record::Gain {
                slot: 1,
                value: 0.5,
            },
            Record::Save {
                slot: 2,
                id: "20260816-143052-271".to_string(),
            },
        ]);
        assert_eq!(notes.len(), 2, "the skipped save was not named: {notes:?}");
        assert!(
            notes[0].starts_with("2 records after the last tick"),
            "{notes:?}"
        );
        assert!(
            notes[1].contains("slot 2") && notes[1].contains("20260816-143052-271"),
            "a skipped save has to name the slot and the set an operator would \
             go and load: {notes:?}"
        );

        // **The control.** Nothing to name and the count stands alone.
        let counted = trailing_notes(&[Record::Gain {
            slot: 1,
            value: 0.5,
        }]);
        assert_eq!(
            counted.len(),
            1,
            "a trailing record that reaches nothing outside the stream is counted \
             and no more: {counted:?}"
        );
        assert!(counted[0].starts_with("1 record after"), "{counted:?}");
    }

    // -- the clock ---------------------------------------------------------
    //
    // These three came here with `frame` when the frame loop moved to
    // `karakuri-engine`. They never touched a sink and never took a device:
    // they are about [`Clock`], which is this program's and stays here, because
    // reading a clock is the one thing a live run does that a replay must not.

    /// **The interval of a frame that never ran is not lost — the next frame
    /// counts it.**
    ///
    /// **This used to be about a frame that found nowhere to draw**, which was
    /// the only way the clock could go unread: `frame::compose` withheld the
    /// committing closure from a refused frame, so `Clock::steps` was not
    /// called and the interval carried. That is no longer a case at all —
    /// every frame `frame::compose` composes reads the clock, whatever the
    /// sinks answered — and the property it was checking is the same one, now
    /// carrying the gap where the frame loop itself does not run: a paused
    /// event loop, a window the operating system stopped sending redraws to, a
    /// long stall. The arithmetic below never mentioned a sink, which is why
    /// the assertion stands unchanged while its subject moved.
    ///
    /// The claim the frame loop's ordering rests on, and until `steps` could be
    /// told what time it is there was no way to state it: the first version of
    /// this test asserted that the step count did not exceed `MAX_STEPS` (it
    /// cannot: `steps` clamps to it) and that the carry was under one (it is:
    /// `steps` subtracts its own floor). Both survived deleting the body of
    /// `Clock::steps`.
    ///
    /// Two clocks over the same span, one reading it in two frames and one in
    /// a single frame because the other was abandoned, must hand out the same
    /// total. That is what "the time survives" means, and it is false for any
    /// clock that resets `last` somewhere other than a frame that goes ahead.
    #[test]
    fn a_frame_that_never_ran_leaves_its_time_for_the_next_one() {
        let start = Instant::now();
        let ms = |n: u64| start + std::time::Duration::from_millis(n);

        let mut drew_every_frame = Clock::new(start);
        let both =
            u32::from(drew_every_frame.steps(ms(16))) + u32::from(drew_every_frame.steps(ms(32)));

        // The same thirty-two milliseconds, with the frame at 16 ms never run
        // at all: `steps` is not called, so `last` does not move.
        let mut skipped_one = Clock::new(start);
        let one = u32::from(skipped_one.steps(ms(32)));

        assert_eq!(
            one, both,
            "the abandoned frame's interval was dropped rather than carried"
        );
        assert!(both > 0, "thirty-two milliseconds is at least one step");
    }

    /// The carry is what makes that true across a frame rate that does not
    /// divide the step rate: whole steps out, the fraction kept.
    #[test]
    fn the_clock_hands_out_whole_steps_and_keeps_the_fraction() {
        let start = Instant::now();
        let mut clock = Clock::new(start);
        let mut total = 0u32;
        // Sixty frames of 16 ms is 960 ms, and at `DT` per step that is a known
        // number of steps — known well enough that dropping the carry loses
        // several of them.
        for i in 1..=60u64 {
            total += u32::from(clock.steps(start + std::time::Duration::from_millis(i * 16)));
        }
        let expected = (0.960 / f64::from(DT)).floor() as u32;
        assert_eq!(
            total, expected,
            "the fraction between frames was dropped: {total} steps for 960 ms"
        );
    }

    /// And the anti-spiral clamp holds: a stall does not become a catch-up.
    #[test]
    fn a_long_gap_falls_behind_rather_than_catching_up() {
        // **The stream's cap, not this crate's copy of it.** `Clock::steps`
        // clamps to the `MAX_STEPS` above, and a `tick` that named more steps
        // than a reader will accept is unreplayable — so the assertion is
        // against the number the record format publishes, and the two agreeing
        // is what is being checked.
        use karakuri_store::record::MAX_STEPS;

        let start = Instant::now();
        let mut clock = Clock::new(start);
        let steps = clock.steps(start + std::time::Duration::from_secs(5));
        assert_eq!(steps, MAX_STEPS, "five seconds is not four steps' worth");
    }
}

#[cfg(test)]
mod value_tests {
    use super::*;

    /// The error, or `None` if the arguments were accepted. `ParseOutcome`
    /// holds GPU-adjacent config and is not `Debug`, and giving it one just so
    /// a test can print it would be the tail wagging the dog.
    fn parse(args: &[&str]) -> Option<String> {
        parse_args_from(args.iter().map(|s| s.to_string())).err()
    }

    /// Every flag that takes a value refuses a bad one rather than keeping its
    /// default. The silence is the bug: a run that quietly used 240 frames is
    /// indistinguishable from one that honoured the `--frames` that was typed.
    #[test]
    fn a_flag_given_a_value_it_cannot_use_says_so() {
        for (args, expect) in [
            (vec!["--frames", "24O"], "--frames"),
            (vec!["--capacity", "lots"], "--capacity"),
            (vec!["--budget-ms", "soon"], "--budget-ms"),
            (vec!["--size", "1280"], "--size"),
            (vec!["--size", "1280x"], "--size"),
            (vec!["--canvas", "1920"], "--canvas"),
            (vec!["--canvas", "x1080"], "--canvas"),
            (vec!["--param", "turbulence"], "--param"),
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains(expect), "{args:?} -> {err}");
        }
    }

    /// **Zero is a typo, not a size.** Every texture descriptor downstream
    /// takes `max(1)` to stay legal, so `--canvas 1920x0` would have rendered a
    /// frame one texel tall and reported the size it was asked for — the same
    /// silence as a `--frames 24O` that renders 240.
    #[test]
    fn an_extent_with_a_zero_side_is_refused_rather_than_clamped() {
        for args in [
            vec!["--canvas", "1920x0"],
            vec!["--canvas", "0x1080"],
            vec!["--canvas", "0x0"],
            vec!["--size", "0x720"],
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains("zero"), "{args:?} -> {err}");
        }
    }

    /// `--size` is the preview window's, and these three runs have no window.
    ///
    /// Refused rather than ignored **because of what it used to mean**: it was
    /// the render size, so a reader typing `--render out.png --size 1920x1080`
    /// means `--canvas`. Quietly rendering at the default instead would be the
    /// worst of the three outcomes — a PNG at a size nobody asked for, with
    /// nothing on stderr.
    #[test]
    fn the_preview_windows_size_is_refused_where_there_is_no_window() {
        for args in [
            vec!["--render", "out.png", "--size", "1920x1080"],
            vec!["--seq", "frames", "--size", "1920x1080"],
            vec![
                "--replay",
                "s",
                "--render",
                "out.png",
                "--size",
                "1920x1080",
            ],
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains("no window"), "{args:?} -> {err}");
        }
        // And it is accepted wherever a window exists, including beside
        // `--canvas`: the two describe different things and were split so they
        // could be given together.
        assert!(parse(&["--size", "800x600"]).is_none());
        assert!(parse(&["--size", "800x600", "--canvas", "1920x1080"]).is_none());
        assert!(parse(&["--render", "out.png", "--canvas", "1920x1080"]).is_none());
    }

    /// A surface with nobody at it, and one more reason besides.
    ///
    /// The other refusals are "an offscreen run takes no live input". This one
    /// is that plus the sharper version: a port that can rewrite a procedure
    /// mid-render is the opposite of an output that is a function of its
    /// arguments.
    #[test]
    fn an_mcp_port_is_refused_where_there_is_no_run_to_drive() {
        for args in [
            vec!["--mcp", "8000", "--render", "out.png"],
            vec!["--mcp", "8000", "--seq", "frames/"],
            vec!["--mcp", "8000", "--replay", "a", "--render", "out.png"],
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains("--mcp"), "{args:?} -> {err}");
        }
        assert!(parse(&["--mcp", "8000"]).is_none());
        assert!(parse(&["--mcp", "8000", "--watch"]).is_none());
        // A port is a number, and a flag given a value it cannot use says so
        // rather than keeping a default.
        assert!(parse(&["--mcp", "eight-thousand"]).is_some());
    }

    /// A tempo source is a live input, and an offscreen run has none.
    ///
    /// The replay half has its own reason and it is the stronger one: a replay
    /// follows the grid the session recorded, so a live source would be
    /// overwriting the performance it is supposed to be reproducing.
    #[test]
    fn a_tempo_source_is_refused_where_there_is_no_performance_to_follow() {
        for args in [
            vec!["--tempo-source", "helper", "--render", "out.png"],
            vec!["--tempo-source", "helper", "--seq", "frames/"],
            vec![
                "--tempo-source",
                "helper",
                "--replay",
                "a",
                "--render",
                "out.png",
            ],
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains("--tempo-source"), "{args:?} -> {err}");
        }
        assert!(parse(&["--tempo-source", "helper"]).is_none());
        assert!(parse(&["--tempo-source", "helper", "--audio-in", "default"]).is_none());
    }

    /// A recorder that never gets built is refused rather than dropped.
    ///
    /// It used to exit 0 having written the PNG and no session at all — no
    /// warning, no file — because the recorder is only constructed on the path
    /// that opens a window. **The worst shape of failure this program has**: not
    /// a wrong output but a missing one, reported as success, from a flag whose
    /// whole purpose is to leave something behind.
    #[test]
    fn recording_is_refused_where_there_is_no_performance() {
        for args in [
            vec!["--render", "out.png", "--record-session", "s"],
            vec!["--seq", "frames", "--record-session", "s"],
            vec![
                "--replay",
                "a",
                "--render",
                "out.png",
                "--record-session",
                "s",
            ],
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains("--record-session"), "{args:?} -> {err}");
        }
        // A live run is where it belongs, and it does **not** need `--load-set`:
        // the material is saved under `ID-material` and put at the head. The
        // help text said otherwise long after that stopped being true.
        assert!(parse(&["--record-session", "s"]).is_none());
        assert!(parse(&["--record-session", "s", "--load-set", "base"]).is_none());
    }

    /// A replay renders at the size the session recorded, and `--canvas` is
    /// refused **only when there is something to refuse it against**.
    ///
    /// The refusal was unconditional and at parse time, which was wrong for
    /// exactly the streams that need the flag: a session written before this
    /// record existed carries no size, so the flag was rejected with the words
    /// "the session records what it rendered at" and the replay then ran at the
    /// untouched default. That is a regression against the old `--size`, which
    /// could set it.
    #[test]
    fn a_replay_refuses_the_canvas_flag_only_when_the_stream_has_one() {
        let performed = Some((1280, 720));
        let flag = (640, 480);

        assert_eq!(
            replay_canvas(performed, flag, false),
            Ok(((1280, 720), None)),
            "the stream's size, and nothing to say about it"
        );

        let refusal = replay_canvas(performed, flag, true).expect_err("was accepted");
        assert!(refusal.contains("--canvas"), "{refusal}");

        // No record: the flag is the way out, and either way it is named,
        // because a replay at a size nobody can vouch for must not look like a
        // faithful one.
        let (size, note) = replay_canvas(None, flag, true).expect("the flag is allowed");
        assert_eq!(size, flag);
        assert!(note.expect("says so").contains("640x480"));

        let (size, note) = replay_canvas(None, flag, false).expect("falls back");
        assert_eq!(size, flag);
        assert!(note.expect("says so").contains("guess"));

        // And the flag reaches parsing at all, which the old refusal blocked.
        assert!(parse(&["--replay", "s", "--render", "out.png", "--canvas", "640x480"]).is_none());
    }

    /// A flag at the end of the line has no value, and the message says which
    /// flag rather than blaming whatever came before it.
    #[test]
    fn a_flag_with_nothing_after_it_says_which_flag() {
        for flag in [
            "--render",
            "--seq",
            "--frames",
            "--size",
            "--set",
            "--exposure",
        ] {
            let err = parse(&[flag]).unwrap_or_else(|| panic!("`{flag}` alone was accepted"));
            assert!(
                err.contains(flag) && err.contains("needs a value"),
                "{flag} -> {err}"
            );
        }
    }

    /// `--render` with the path forgotten used to open a window: `None` fell
    /// through to the interactive branch, so a batch script with a typo hung
    /// instead of failing.
    #[test]
    fn a_flag_does_not_swallow_the_next_flag() {
        let err = parse(&["--render", "--frames", "10"]).expect("`--render --frames` was accepted");
        assert!(err.contains("--render") && err.contains("not one"), "{err}");
    }

    /// A negative number is a value, not an option — the check is "looks like
    /// a flag", and `-1.5` does not.
    #[test]
    fn a_negative_number_still_reads_as_a_value() {
        let err = parse(&["--exposure", "-1.5"]).expect("a negative exposure was accepted");
        assert!(
            err.contains("positive"),
            "rejected for the wrong reason: {err}"
        );
    }
}

/// **What a live save actually writes**, through a Set that has been built and
/// then played with.
///
/// Everything else about saving is checked against flags — see the `--save-set`
/// tests above and `setfile`'s own. The claim a *live* save makes is a
/// different one and no flag can stand in for it: **what reaches the file is
/// what is on screen**, after a parameter has moved and after a build has been
/// rolled back. So these build a real Set, disturb it the way a run disturbs
/// one, and read the file back through the loader `--load-set` uses.
#[cfg(test)]
mod live_save_tests {
    use super::*;
    // The card writer, reached only by the test that pins what it says when a
    // disk refuses it.
    use karakuri_environment::meta::put_meta;
    use std::path::Path;

    /// Integration tests get the *package* as their working directory, and unit
    /// tests get it too — so `examples/` is two levels up from here.
    fn workspace() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root")
    }

    fn example(name: &str) -> String {
        std::fs::read_to_string(workspace().join("examples").join(name)).expect("an example")
    }

    /// Write `src` into `dir` and hand back the path, so a test can edit a
    /// procedure the way an operator does — by replacing the file.
    fn kir(dir: &tempfile::TempDir, name: &str, src: &str) -> PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, src).expect("write");
        path
    }

    /// One slot's files, compiled and sorted exactly as a run compiles them —
    /// so `Placed` here carries the layers and indices the real thing carries.
    fn slot(paths: &[PathBuf]) -> (Material, Vec<Placed>) {
        let rest: Vec<Named> = paths[1..].iter().cloned().map(Named::bare).collect();
        sort_slot(0, &Named::bare(paths[0].clone()), &rest)
    }

    /// A Set built from that material, at the capacities, salts and camera
    /// given — which is what a run hands `build`, and what a save has to read
    /// back off the Set rather than off these arguments.
    ///
    /// **`camera` is an `Option` because `build`'s is**: `None` is a slot no
    /// `camera` record reached, which leaves the Set at the built-in orbit's
    /// defaults. See `recorded_camera`.
    fn set_of(
        gpu: &Gpu,
        material: &Material,
        capacities: &[u32],
        salts: &[u32],
        overrides: &[ParamWrite],
        camera: Option<karakuri_engine::camera::Orbit>,
    ) -> Set {
        let mut attached = vec![false; 0];
        build(
            gpu,
            &material.l1s,
            &material.l2s,
            &material.l3s,
            &material.fields,
            &material.l4s,
            karakuri_engine::set::Layering::Overdraw,
            &material.names,
            &[],
            capacities,
            &mut attached,
            overrides,
            &[],
            &[],
            salts[0],
            salts,
            camera,
            // No selection: these Sets overdraw, and a fold nobody built has
            // no renderer to be folded to. See `recorded_live`.
            None,
        )
    }

    /// **What a run does to one slot before its first frame**: the slot
    /// recorded as running the material that was compiled for it.
    ///
    /// Through [`Running::at_launch`] rather than around it, for the reason
    /// [`save_and_load`] goes through [`Save::run`]: assembling the seeding by
    /// hand here would be a second copy of it, and the whole of what these tests
    /// are about is that there is only one.
    ///
    /// **No store root, because there is no store.** A windowed run touches one
    /// when a save happens and not before — see [`Running::at_launch`].
    fn launched(placed: &[Placed]) -> Running {
        Running::at_launch(&[placed.to_vec()], 1)
    }

    /// One node as the watcher hands it over when a build lands: the source in
    /// the store, and its address beside the hash.
    fn stored(store: &karakuri_store::store::Store, layer: &'static str, src: &str) -> Landed {
        (layer, 0, store.put_artifact(src.as_bytes()).expect("put"))
    }

    type Landed = (&'static str, u32, karakuri_store::hash::Hash);

    /// Gather, write, and read back — the whole of what pressing `k` does,
    /// minus the thread and the channel.
    ///
    /// **Through [`Save::run`] rather than around it**, so that what a test
    /// exercises is the function the spawned thread calls. Assembling the same
    /// three steps by hand here would be a second copy of the save, and a test
    /// of a copy is a test of nothing.
    fn save_and_load(store_root: &Path, id: &str, set: &Set, sources: Sources) -> setfile::Loaded {
        Save {
            slot: 0,
            asked: Asked::Operator,
            id: id.to_string(),
            root: store_root.to_path_buf(),
            sources,
            values: playing_values(set, &[]),
        }
        .run()
        .expect("the Set file is written");
        let store = karakuri_store::store::Store::open(store_root).expect("store");
        setfile::load(&store, id).expect("and reads back")
    }

    /// **A geometry with a negative default**, which is the declaration the
    /// fold exists for and the one no example in the tree carries.
    ///
    /// `drift_shell` with one param added and read, so the procedure still
    /// compiles, still draws, and now declares a default that is `Unary { Neg,
    /// Lit }` rather than a literal.
    fn signed_l1() -> String {
        let src = example("drift_shell.kir");
        let with_param = src.replace(
            "  param drift      : float [0.0, 2.0] = 0.6",
            "  param drift      : float [0.0, 2.0] = 0.6\n  \
             param signed     : float [-1.0, 1.0] = -0.35",
        );
        assert_ne!(with_param, src, "the example's params moved");
        let with_use = with_param.replace(
            "    position = p;",
            "    position = p + vec3(signed, 0.0, 0.0);",
        );
        assert_ne!(with_use, with_param, "the example's element block moved");
        with_use
    }

    /// **A stored artifact has a card beside it, and the card says what the
    /// `.kir` declares** — every param with its range, the capacity range and
    /// its default, and the emitted attributes.
    ///
    /// `docs/ir-spec.md`'s metadata section long described records that nothing
    /// wrote and nothing read, so every claim it made was unenforced prose. This is the half a compile pass can produce, pinned against a
    /// source a reader can check it against by eye.
    ///
    /// **Including a negative default**, which is the one declaration a second
    /// reader of defaults gets wrong — see
    /// [`the_engine_and_the_metadata_writer_cannot_disagree_about_a_default`],
    /// which is the other half of that and needs a GPU. This one needs none:
    /// putting an artifact is a compile and two files.
    ///
    /// **Through [`Placed::put`] rather than around it.** That is the funnel
    /// every stored artifact goes through — the watcher's builds, a recorded
    /// run's seeding, and `--save-set` — so a test that assembled the write by
    /// hand would be a test of a copy.
    #[test]
    fn a_stored_artifact_has_a_card_saying_what_its_source_declares() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = vec![
            kir(&dir, "a.kir", &signed_l1()),
            kir(&dir, "r.kir", &example("soft_points.kir")),
        ];
        let (_material, placed) = slot(&paths);
        let store =
            karakuri_store::store::Store::open(dir.path().join("store")).expect("a store opens");

        let hash = placed[0].put(&store).expect("the artifact is stored");
        let card = store
            .read_meta(&hash)
            .expect("an artifact that was put has a card beside it");
        let records: Vec<Record> = card.iter().map(|l| l.record().clone()).collect();

        assert_eq!(
            records.first(),
            Some(&Record::Meta {
                hash,
                name: "drift_shell".to_string(),
                kind: Layer::L1,
                v: 1,
            }),
            "a card's head names the artifact it describes and the procedure's \
             own name"
        );

        let declared: Vec<(&str, f32, f32, Option<f32>)> = records
            .iter()
            .filter_map(|r| match r {
                Record::ParamDecl {
                    key,
                    ty,
                    min,
                    max,
                    default,
                } => {
                    assert_eq!(ty, "float", "`{key}` is declared a float in the source");
                    Some((key.as_str(), *min, *max, *default))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            declared,
            vec![
                ("radius", 0.1, 8.0, Some(2.6)),
                ("turbulence", 0.0, 3.0, Some(1.1)),
                ("swirl", 0.0, 2.0, Some(0.35)),
                ("drift", 0.0, 2.0, Some(0.6)),
                // The one the fold exists for: `= -0.35` is a negation of a
                // literal, and a card that read `Expr::Lit` alone would carry
                // no `default` here at all.
                ("signed", -1.0, 1.0, Some(-0.35)),
            ],
            "the params a card declares are not the source's, in the source's order"
        );

        assert!(
            records.contains(&Record::CapacityDecl {
                min: 4096,
                max: 1048576,
                default: 262144,
            }),
            "the declared capacity range is not on the card: {records:?}"
        );
        assert!(
            records.contains(&Record::Emit {
                attrs: vec![
                    "position".to_string(),
                    "velocity".to_string(),
                    "age".to_string(),
                ],
            }),
            "the emitted attributes are not on the card: {records:?}"
        );
    }

    /// **A card that will not write is said out loud and does not fail the
    /// save.**
    ///
    /// The policy [`Placed::put`] states: the artifact is the thing, its card is
    /// derived from the `.kir` plus a compile pass and regenerates on the next
    /// one, so a store holding the source and no card holds everything that
    /// cannot be recovered — and failing the put instead would lose an operator
    /// a save over a file nothing has read yet. Both halves were prose until
    /// here, and the half that rots quietly is the second: a `put` that started
    /// returning the card's error would be caught by any test that saves, while
    /// a `put_meta` that swallowed it would be caught by none.
    ///
    /// **The card's own path taken by a directory**, because that is the one
    /// failure that reaches the card and nothing else. A read-only store root
    /// would fail `put_artifact` first and prove the opposite thing. The path is
    /// spelled here the way `Store::meta_path` spells it — it is private — as
    /// `karakuri-store`'s own metadata tests spell it.
    #[test]
    fn a_card_that_will_not_write_is_said_and_does_not_fail_the_save() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = vec![
            kir(&dir, "a.kir", &example("drift_shell.kir")),
            kir(&dir, "r.kir", &example("soft_points.kir")),
        ];
        let (_material, placed) = slot(&paths);
        let root = dir.path().join("store");
        let store = karakuri_store::store::Store::open(&root).expect("a store opens");

        let hash = placed[0].hash();
        std::fs::create_dir(root.join(format!("{}.meta.ndjson", hash.short(64))))
            .expect("the card's path is taken");

        assert_eq!(
            placed[0]
                .put(&store)
                .expect("the save is not failed by a card"),
            hash,
            "a put that survived its card came back with another artifact"
        );
        assert!(
            store.get_artifact(&hash).is_ok(),
            "the source did not reach the store, so nothing here was recoverable"
        );
        assert!(
            store.read_meta(&hash).is_err(),
            "the card was written after all, and this test proves nothing"
        );

        // The same call the put makes, read back rather than watched on a
        // terminal — see [`put_meta`], which returns what it printed for this.
        let said = put_meta(&store, &hash, &placed[0].meta)
            .expect("a card that would not write is reported");
        assert!(
            said.contains(&hash.short(12)),
            "the operator is not told which artifact: {said}"
        );
        assert!(
            said.contains("the artifact is stored"),
            "the operator is not told the save survived: {said}"
        );
    }

    /// **A rollback onto the launch version restores the launch addresses** —
    /// the value a `procedure` record would be written from, not the record.
    ///
    /// The name used to claim the stream, which this cannot reach: writing a
    /// record needs a `Live`, and a `Live` needs a window and a GPU. What it
    /// pins is one step earlier and is the step that was wrong — on the *first*
    /// rollback of a slot there was nothing to write a record from: `previous`
    /// was seeded from nothing, the restore produced `None`, and
    /// `Live::took_up` returned before reaching a record. The stream then said
    /// nothing at all about a swap that had visibly gone out, and a replay
    /// meeting it went on drawing the refused build.
    ///
    /// So the claim here is that the restore names the **launch** sources, not
    /// the refused edit that is on disk and was on screen a frame ago. That
    /// those addresses resolve in a recorded run's store is the other half, and
    /// it is [`a_recorded_run_puts_its_launch_sources_where_a_replay_looks`].
    ///
    /// No GPU: nothing here builds a Set, and nothing here opens a store —
    /// which is itself the point of [`Running::at_launch`] no longer doing so.
    #[test]
    fn a_rollback_onto_the_launch_version_restores_the_launch_addresses() {
        let dir = tempfile::tempdir().expect("tempdir");
        let at_start = example("drift_shell.kir");
        let renderer = example("soft_points.kir");
        let paths = vec![kir(&dir, "a.kir", &at_start), kir(&dir, "r.kir", &renderer)];
        let (_material, placed) = slot(&paths);
        let root = dir.path().join("store");

        let mut running = launched(&placed);
        // **What those bytes hash to, without putting them anywhere.** The
        // address is a function of the source and of nothing else — see
        // [`Placed::hash`] — so a test can state the expected one without a
        // store, which is exactly the property a windowed run leans on.
        let launch_l1 = karakuri_store::hash::Hash::of(at_start.as_bytes());
        let launch_l4 = karakuri_store::hash::Hash::of(renderer.as_bytes());

        // The operator edits, it compiles, the watcher stores it and the swap
        // lands. The file on disk is the edit from here on.
        let store = karakuri_store::store::Store::open(&root).expect("store");
        let refused = at_start.replace("proc drift_shell", "proc refused_edit");
        std::fs::write(&paths[0], &refused).expect("the edit");
        running.landed(
            0,
            Some(vec![
                stored(&store, "L1", &refused),
                stored(&store, "L4", &renderer),
            ]),
        );

        // The governor refuses it. **The first rollback of this slot**: nothing
        // swapped in before the build that just went out, which is the case
        // that used to fall through to no record at all.
        let restored = running
            .rolled_back(0)
            .expect("a rollback onto the launch version came back as nothing to say");

        assert_eq!(
            restored,
            vec![("L1", 0, launch_l1), ("L4", 0, launch_l4)],
            "the stream would name something other than the launch sources the \
             slot fell back onto"
        );
        assert!(
            std::fs::read_to_string(&paths[0])
                .expect("the file")
                .contains("proc refused_edit"),
            "the path still has to hold the refused version, or this test is \
             checking nothing"
        );
    }

    /// **A build whose sources never reached the store is still a swap.**
    ///
    /// The watcher compiles, fails to `put_artifact`, says so, and returns the
    /// `Request` anyway — so the build goes on screen with no address anybody
    /// can name. `Live::took_up` used to answer that by returning before
    /// touching [`Running`] at all, while the matching rollback went through
    /// `rolled_back` unconditionally. **Half a transition rule**: the swap did
    /// not move `playing` into `previous` and the rollback moved `previous`
    /// back anyway, so one such pair left every later reader one generation
    /// behind the screen. `k` wrote that version down and a recorded run put
    /// `procedure` records naming it into the stream, with nothing anywhere
    /// saying the file and the picture had come apart.
    ///
    /// Two shapes, and the second is the worse one:
    ///
    /// - **mid-run**, where the slot skews by a generation: launch, build A
    ///   lands, build B is unstored, B is refused for cost — the screen is back
    ///   on A and so must the slot be, where it used to read as the launch
    ///   version;
    /// - **on the first build**, where `previous` is still `None`, so the
    ///   rollback left the slot with no address at all and `k` refused for the
    ///   rest of the run on the grounds that the launch sources were not in the
    ///   store — which was false, and the operator had no way to find that out.
    ///
    /// No GPU and no window: the transition is the whole subject, which is what
    /// [`Running`] being its own type is for.
    #[test]
    fn a_build_whose_sources_never_reached_the_store_is_still_a_swap() {
        let dir = tempfile::tempdir().expect("tempdir");
        let at_start = example("drift_shell.kir");
        let renderer = example("soft_points.kir");
        let paths = vec![kir(&dir, "a.kir", &at_start), kir(&dir, "r.kir", &renderer)];
        let (_material, placed) = slot(&paths);
        let root = dir.path().join("store");
        let store = karakuri_store::store::Store::open(&root).expect("store");

        let launch: Vec<Landed> = vec![
            ("L1", 0, karakuri_store::hash::Hash::of(at_start.as_bytes())),
            ("L4", 0, karakuri_store::hash::Hash::of(renderer.as_bytes())),
        ];

        let mut running = launched(&placed);
        assert_eq!(
            running.playing(0),
            Some(&launch),
            "the slot did not start on what it launched with, so nothing below \
             is about the transition"
        );

        // Build A: it compiled, the watcher stored it, the swap landed.
        let build_a = at_start.replace("proc drift_shell", "proc build_a");
        let a: Vec<Landed> = vec![
            stored(&store, "L1", &build_a),
            stored(&store, "L4", &renderer),
        ];
        running.landed(0, Some(a.clone()));

        // Build B: it compiled, the store refused it, and the deck installed it
        // regardless — which is what `watch::Watch::poll` does, and what makes
        // this a state the run can actually be in.
        running.landed(0, None);
        assert!(
            running.playing(0).is_none(),
            "a slot showing a version nothing can name reported an address, and \
             whatever it reported is not what is on screen"
        );

        // The governor refuses B for cost. The screen goes back to A.
        let restored = running.rolled_back(0);
        assert_eq!(
            restored.as_ref(),
            Some(&a),
            "the rollback restored a generation further back than the screen did"
        );
        assert_eq!(
            running.playing(0),
            Some(&a),
            "the slot reads as running something other than the build the deck \
             put back — a save and a `procedure` record would both name it"
        );

        // **The same rule on the first build of a run**, where the skew took
        // the slot's address away entirely rather than moving it.
        let mut first = launched(&placed);
        first.landed(0, None);
        first.rolled_back(0);
        assert_eq!(
            first.playing(0),
            Some(&launch),
            "a rollback onto the launch version left the slot unsavable for the \
             rest of the run, and the refusal said the launch sources were not \
             in the store"
        );
    }

    /// **A windowed run creates no store until something is saved.**
    ///
    /// `docs/manual.md` promises that a run which cannot be edited "copies
    /// nothing and creates no directory", and the launch seeding falsified it:
    /// it opened the store — which `create_dir_all`s the root and its
    /// subdirectories — and wrote one artifact per node, so a plain
    /// `karakuri-cli examples/...` left a `.karakuri` behind in whatever
    /// directory it was run from.
    ///
    /// **The seeding costs nothing because an address is not a file.** The
    /// second assertion is what makes the first mean something: the slot knows
    /// exactly what it is running, it simply has not written it anywhere. The
    /// bytes reach the store at the moment a file names them — see
    /// [`Sources::into_nodes`], which
    /// [`a_rewrite_between_the_compile_and_the_first_frame_cannot_reach_a_save`]
    /// reads back off the disk.
    #[test]
    fn a_windowed_run_creates_no_store_until_something_is_saved() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = vec![
            kir(&dir, "a.kir", &example("drift_shell.kir")),
            kir(&dir, "r.kir", &example("soft_points.kir")),
        ];
        let (_material, placed) = slot(&paths);
        let root = dir.path().join("store");

        let running = launched(&placed);
        assert!(
            !root.exists(),
            "a run that was asked to draw a window and nothing else created a \
             store, which `docs/manual.md` promises it does not"
        );
        assert_eq!(
            running.playing(0).map(Vec::len),
            Some(2),
            "the slot has no address, so `k` would refuse — which is not the \
             way to create no directory"
        );
    }

    /// **A recorded run puts every slot's launch sources where a replay looks.**
    ///
    /// The one reader that needs those bytes on disk before anything is saved.
    /// A rollback onto the launch version emits `procedure` records naming its
    /// hashes — see
    /// [`a_rollback_onto_the_launch_version_restores_the_launch_addresses`] —
    /// and a replay rebuilds by reading each back out of the store. A record
    /// naming bytes nobody kept is the same silence as no record at all.
    ///
    /// **Slot 1, deliberately.** `session_head` writes slot 0's material and
    /// says out loud that a session stream cannot describe a deck, so slot 0
    /// would pass on the head's I/O alone and prove nothing about the seeding.
    /// A `procedure` record, unlike a head, does address any slot.
    ///
    /// Paired with its control: nothing is in the store before the seeding, so
    /// this is about the seeding rather than about a store that had the bytes
    /// from somewhere else.
    #[test]
    fn a_recorded_run_puts_its_launch_sources_where_a_replay_looks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let at_start = example("drift_shell.kir");
        let renderer = example("soft_points.kir");
        // Slot 1's geometry is a different procedure, so the two slots are two
        // addresses rather than one shared by a content-addressed store.
        let other = at_start.replace("proc drift_shell", "proc slot_one_shell");
        let first = vec![kir(&dir, "a.kir", &at_start), kir(&dir, "r.kir", &renderer)];
        let second = vec![kir(&dir, "b.kir", &other), kir(&dir, "s.kir", &renderer)];
        let (_m0, slot0) = slot(&first);
        let (_m1, slot1) = slot(&second);
        let placed = vec![slot0, slot1];

        let root = dir.path().join("store");
        let store = karakuri_store::store::Store::open(&root).expect("store");
        // **The control.** Opening a store does not fill it, so everything
        // below is about the seeding.
        for nodes in &placed {
            for node in nodes {
                assert!(
                    store.get_artifact(&node.hash()).is_err(),
                    "the store already held this run's sources, so the seeding \
                     below could not be what put them there"
                );
            }
        }

        seed_store_for_replay(&store, &placed);

        // Slot 1 takes a build and the governor refuses it, which is the state
        // that makes the stream name the launch version.
        let mut running = Running::at_launch(&placed, 2);
        let refused = other.replace("proc slot_one_shell", "proc refused_edit");
        running.landed(
            1,
            Some(vec![
                stored(&store, "L1", &refused),
                stored(&store, "L4", &renderer),
            ]),
        );
        let restored = running
            .rolled_back(1)
            .expect("a rollback onto the launch version came back as nothing to say");
        for (_, _, hash) in &restored {
            assert!(
                store.get_artifact(hash).is_ok(),
                "a `procedure` record for slot 1 names a source this session's \
                 replay cannot resolve, so the replay rebuilds nothing where the \
                 run showed the version it launched with"
            );
        }
    }

    /// **Every `Kind` survives the round trip a saved node makes.**
    ///
    /// A node's layer is written out by [`setfile::kind_name`] and read back by
    /// [`layer_named`], and nothing pinned them as inverses. A sixth `Kind`
    /// would have compiled — `kind_name`'s match is exhaustive and would be
    /// updated, `layer_named`'s ends in a wildcard and would not — and surfaced
    /// as `a node on layer ... cannot be saved` at the moment an operator
    /// pressed `k`, which is the worst place in the program to find out.
    ///
    /// **The `match` is what makes this exhaustive**, not the array. A list can
    /// fall one short in silence; a sixth variant stops this file compiling,
    /// which is the failure that was wanted in place of the runtime one.
    #[test]
    fn every_kind_survives_the_round_trip_a_saved_node_makes() {
        use karakuri_ir::Kind;
        for kind in [Kind::L1, Kind::L2, Kind::L3, Kind::L4, Kind::Field] {
            match kind {
                Kind::L1 | Kind::L2 | Kind::L3 | Kind::L4 | Kind::Field => {}
            }
            assert_eq!(
                layer_named(setfile::kind_name(kind)),
                Some(kind),
                "a node on this layer is written into a Set file under a name \
                 nothing reads back, so saving it refuses at the key press"
            );
        }
    }

    /// **A slot with nothing in the store behind it saves nothing**, rather than
    /// writing a file describing no Set.
    ///
    /// The `--load-set` without `--watch` case: the material came out of the
    /// store by hash with no files behind it, and `placed` is empty for that
    /// slot. There is no version to name, which is a different thing from
    /// naming the wrong one, and `Live::save_set` refuses it by name — see the
    /// refusal there, which says which set the run came from and what would
    /// make the slot savable.
    #[test]
    fn a_slot_with_no_sources_of_its_own_has_nothing_to_save() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("store");
        let running = Running::at_launch(&[Vec::new()], 1);
        assert!(
            live_sources(running.playing(0), &[]).is_empty(),
            "a slot with no files behind it has no sources to write"
        );
        assert!(
            !root.exists(),
            "a deck with no files behind it opened a store to write nothing into"
        );
    }

    /// **The two refusals a save can meet name the way out of them.**
    ///
    /// Both reach a model now as well as a terminal, which is what makes them
    /// worth a test: `Live::save_set` needs a window and a GPU, so the sentences
    /// live in free functions and this is where they are checked. The `--mcp`
    /// half is a correction that has already been made once — this refusal named
    /// `--watch` alone for as long as `--mcp` also made a slot savable, and sent
    /// operators to restart a set for a flag they did not need.
    #[test]
    fn a_save_that_cannot_happen_says_why_and_what_would_change_it() {
        let loaded = nothing_to_save(2, Some("night01"), true);
        assert!(loaded.contains("`night01`"), "{loaded}");
        assert!(loaded.contains("--watch"), "{loaded}");
        assert!(
            loaded.contains("--mcp"),
            "a slot `--mcp` alone would have made savable was told to restart for \
             `--watch`: {loaded}"
        );

        // A slot that has files behind it and nothing in the store is a
        // different fact, and neither flag is the answer to it.
        let unstored = nothing_to_save(2, Some("night01"), false);
        assert!(
            !unstored.contains("night01"),
            "a slot with files of its own was told it came out of a set file: {unstored}"
        );

        // **The range, said without underflowing on a deck with no slots.** The
        // arm that cannot happen is the one that stops saying so quietly.
        assert_eq!(no_such_slot(9, 4), "no slot 9: this deck holds slots 0-3");
        assert!(no_such_slot(0, 0).contains("holds none"));
    }

    /// **A renderer the slot does not draw with, refused in the shared words**,
    /// and refused at the boundary rather than one past it.
    ///
    /// The sentence is pinned with an `assert_eq!` against
    /// [`no_such_renderer`] rather than a `contains`, which is the lesson
    /// [`no_such_slot`] paid for: four spellings of one refusal lived side by
    /// side because every test asked only whether the range appeared in it.
    /// Both surfaces that can meet this — the key press and a replayed `select`
    /// record — go through [`renderer_in_range`], so pinning it here pins both.
    #[test]
    fn a_renderer_a_slot_does_not_draw_with_is_refused_with_the_range_it_missed() {
        assert_eq!(
            renderer_in_range(1, 3, 3).expect_err("renderer 3 of three"),
            "no renderer 3: slot 1 draws with renderers 0-2"
        );
        // The boundary either side of it, which is where the off-by-one would
        // live: the last renderer is `count - 1` and it is legal.
        assert!(renderer_in_range(1, 2, 3).is_ok());
        assert!(renderer_in_range(0, 0, 1).is_ok());
        assert!(renderer_in_range(0, 1, 1).is_err());
        // And the arm that cannot happen, said without underflowing.
        assert_eq!(
            no_such_renderer(0, 0, 0),
            "no renderer 0: slot 0 draws with none"
        );
    }

    /// This file's own source, at compile time. The scanner below reads
    /// [`Live::key`] out of it rather than being told what the keys are — the
    /// shape `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs` set: read
    /// the checked-in source, and carry a floor so the scan cannot silently
    /// match nothing.
    const SOURCE: &str = include_str!("main.rs");

    /// How [`BINDINGS`] spells the arms whose pattern is a name rather than a
    /// character. Nothing in `Key::Named(NamedKey::Escape)` says `esc`, so this
    /// one mapping cannot be derived and is stated — but it is stated as a
    /// *table*, and a `NamedKey` arm missing from it fails
    /// [`every_key_the_live_path_acts_on_is_documented`] by name rather than
    /// being passed over. That is the whole difference from the list of
    /// characters that used to be here: a named key added tomorrow is a test
    /// failure that says which key, not a silence.
    const NAMED_KEY_SPELLINGS: &[(&str, &str)] = &[("Escape", "esc"), ("Space", "space")];

    /// The end of the character literal starting at `at`, or `None` if what is
    /// there is not one.
    ///
    /// A lifetime has to be told from a literal — `'a` is one, `'a'` and `'\n'`
    /// are the other — because getting it wrong lets a `'"'` open a string that
    /// swallows the rest of the file. `'\''` is why an escape cannot simply
    /// look for the next quote: the escaped quote *is* the next quote.
    fn char_literal_end(src: &str, at: usize) -> Option<usize> {
        let b = src.as_bytes();
        if b.get(at) != Some(&b'\'') {
            return None;
        }
        if b.get(at + 1) == Some(&b'\\') {
            // A two-byte escape — `\\`, `\'`, `\n`, `\0` — closes at `at + 3`.
            if b.get(at + 3) == Some(&b'\'') {
                return Some(at + 4);
            }
            // `\x41`, `\u{2026}`: the first quote after the escape. Bounds
            // first, so a file ending mid-literal is `None` and not a panic.
            if at + 3 > src.len() {
                return None;
            }
            return src[at + 3..].find('\'').map(|n| at + 3 + n + 1);
        }
        let c = src[at + 1..].chars().next()?;
        let close = at + 1 + c.len_utf8();
        (b.get(close) == Some(&b'\'')).then_some(close + 1)
    }

    /// The character a literal's inside spells, the way rustc reads it.
    ///
    /// Panics rather than returning nothing on a spelling it does not know: a
    /// key quietly dropped here is a key quietly undocumented, which is the
    /// exact failure this file is closing.
    fn unescape(lit: &str) -> char {
        let mut cs = lit.chars();
        let first = cs.next().expect("a character literal is not empty");
        if first != '\\' {
            return first;
        }
        match cs.next().expect("an escape has a second character") {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '0' => '\0',
            c @ ('\\' | '\'' | '"') => c,
            _ => panic!("`'{lit}'` is a key spelling this scanner cannot read"),
        }
    }

    /// Comments and string literals blanked to spaces, keeping length and line
    /// structure. Character literals are left exactly as written — they are the
    /// payload.
    ///
    /// Load-bearing in both directions. Comments are prose and prose is full of
    /// apostrophes; string literals hold `"{BINDINGS}"` and every message the
    /// arms print. Cut down from the same function in
    /// `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs`; this file has no
    /// block comments and no raw strings, and if one arrives the floors below
    /// are what refuses the mis-scan.
    fn blank_comments_and_strings(src: &str) -> String {
        let b = src.as_bytes();
        let mut out = b.to_vec();
        let mut i = 0;
        while i < b.len() {
            if b[i..].starts_with(b"//") {
                let end = src[i..].find('\n').map_or(b.len(), |n| i + n);
                for c in out[i..end].iter_mut() {
                    *c = b' ';
                }
                i = end;
                continue;
            }
            if b[i] == b'"' {
                let start = i;
                i += 1;
                while i < b.len() {
                    match b[i] {
                        b'\\' => i += 2,
                        b'"' => {
                            i += 1;
                            break;
                        }
                        _ => i += 1,
                    }
                }
                let end = i.min(b.len());
                for c in out[start..end].iter_mut() {
                    if *c != b'\n' {
                        *c = b' ';
                    }
                }
                continue;
            }
            if let Some(end) = char_literal_end(src, i) {
                i = end;
                continue;
            }
            i += 1;
        }
        String::from_utf8(out).expect("blanking only ever writes spaces")
    }

    /// [`Live::key`]'s body, blanked, found by its signature.
    ///
    /// The signature and not a line number, and not the name alone — `key` is a
    /// common word. It is spelled with `concat!` so the joined needle exists
    /// nowhere in this file except the function it names: a source scanner that
    /// finds itself is the classic way one of these comes back green. That it
    /// occurs exactly once is asserted, so a renamed or reformatted signature
    /// fails here instead of leaving the scan with nothing to read.
    fn live_key_body() -> String {
        let blanked = blank_comments_and_strings(SOURCE);
        let needle = concat!("fn ", "key(&mut self, key: &Key) -> bool {");
        assert_eq!(
            blanked.matches(needle).count(),
            1,
            "`{needle}` is not in this file exactly once — the scanner is \
             reading nothing, or reading the wrong function"
        );
        let open = blanked.find(needle).expect("just counted one") + needle.len();
        let b = blanked.as_bytes();
        let (mut i, mut depth) = (open, 1usize);
        let end = loop {
            if i >= b.len() {
                panic!("`{needle}` never closes");
            }
            // A braced character literal is a key like any other: stepped over
            // whole, so binding `{` could not open a block here.
            if let Some(skip) = char_literal_end(&blanked, i) {
                i = skip;
                continue;
            }
            match b[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break i;
                    }
                }
                _ => {}
            }
            i += 1;
        };
        blanked[open..end].to_string()
    }

    /// One arm of [`Live::key`]'s match, as its pattern reads.
    enum Arm {
        /// A character it acts on: `'x' => …`.
        Char(char),
        /// A span of them: `'0'..='3' => …`.
        Range(char, char),
        /// A `NamedKey`, whose pattern is a name and not a character at all.
        Named(String),
    }

    /// Every arm of [`Live::key`], read off the pattern side of each `=>` in
    /// its body.
    ///
    /// The pattern side and not the line, because everything after the first
    /// `=>` is the arm's *body* — where `unwrap_or('\0')` lives, and `\0` is not
    /// a binding. Line by line, because a pattern and its `=>` share a line; an
    /// arm body that grew an `=>` of its own would be read as a pattern, which
    /// can only ever demand documentation for a key nobody binds, and that
    /// fails loudly rather than passing quietly.
    fn live_key_arms(body: &str) -> Vec<Arm> {
        let mut arms = Vec::new();
        for line in body.lines() {
            let Some(cut) = line.find("=>") else {
                continue;
            };
            let pattern = &line[..cut];
            for (at, marker) in pattern.match_indices("NamedKey::") {
                let name: String = pattern[at + marker.len()..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    arms.push(Arm::Named(name));
                }
            }
            let mut chars = Vec::new();
            let mut i = 0;
            while i < pattern.len() {
                match char_literal_end(pattern, i) {
                    Some(end) => {
                        chars.push(unescape(&pattern[i + 1..end - 1]));
                        i = end;
                    }
                    None => i += 1,
                }
            }
            // `'0'..='3'` is one binding spanning four keys, not two bindings.
            if pattern.contains("..=") {
                assert_eq!(
                    chars.len(),
                    2,
                    "`{pattern}` is a range with {} endpoints",
                    chars.len()
                );
                arms.push(Arm::Range(chars[0], chars[1]));
            } else {
                arms.extend(chars.into_iter().map(Arm::Char));
            }
        }
        arms
    }

    /// The key column of [`BINDINGS`]: two spaces, the keys, then the gap
    /// before the description. Keys are documented in pairs where they come in
    /// pairs — `[ ]`, `u i`, `h ?` — so it is the column that is read and not
    /// the first character of a line.
    ///
    /// A column and never a substring of the whole constant, which is the trap
    /// this text is laid out to defuse: `?` occurs in the prose of the `, .`
    /// line, so `BINDINGS.contains("?")` is true whether or not `?` is bound to
    /// anything. See [`a_key_named_only_in_prose_is_not_documented`].
    fn documented_keys(bindings: &str) -> Vec<&str> {
        bindings
            .lines()
            .filter_map(|line| line.strip_prefix("  "))
            .filter(|line| !line.starts_with(' '))
            .flat_map(|line| line.split("  ").next().unwrap_or_default().split(' '))
            .filter(|key| !key.is_empty())
            .collect()
    }

    /// **Every key `Live::key` acts on is in [`BINDINGS`]**, which is the only
    /// thing standing between a control and being undiscoverable: there is no
    /// on-screen UI, and this text is both what `--help` prints and what `h`
    /// does.
    ///
    /// The keys are *read out of the match arms* — see [`live_key_body`] — and
    /// not restated here. The version of this test that restated them iterated
    /// a hard-coded array of 32 characters, so what it enforced was "these 32
    /// keys are documented"; `'h' | '?'` had been a live arm with no entry in
    /// the column the whole time and this test passed on every run. **A check
    /// weaker than its own name is worse than no check**, because the name is
    /// what stops anyone looking again — nobody re-reads a green
    /// `every_key_the_live_path_acts_on_is_documented`.
    ///
    /// Which is also why the counts are asserted. A scanner whose pattern
    /// stops matching finds nothing and then passes everything, and that is the
    /// same failure a second time.
    #[test]
    fn every_key_the_live_path_acts_on_is_documented() {
        let documented = documented_keys(BINDINGS);
        let (mut chars, mut ranges, mut named) = (0, 0, 0);

        for arm in live_key_arms(&live_key_body()) {
            match arm {
                Arm::Char(c) => {
                    chars += 1;
                    let key = c.to_string();
                    assert!(
                        documented.contains(&key.as_str()),
                        "`{c}` is a key `Live::key` acts on and has no entry in the \
                         bindings — an operator has no way to find it"
                    );
                }
                // Documented as the span it is, `0-3`, and the span is built
                // from the arm's own endpoints rather than being spelled here.
                Arm::Range(from, to) => {
                    ranges += 1;
                    let key = format!("{from}-{to}");
                    assert!(
                        documented.contains(&key.as_str()),
                        "`{key}` is a range `Live::key` acts on and has no entry in \
                         the bindings"
                    );
                }
                // A named arm carries no character to look for, so its spelling
                // comes from the one table there is — and an unknown name stops
                // the run rather than being skipped.
                Arm::Named(name) => {
                    named += 1;
                    let (_, spelling) = NAMED_KEY_SPELLINGS
                        .iter()
                        .find(|(known, _)| *known == name)
                        .unwrap_or_else(|| {
                            panic!(
                                "`NamedKey::{name}` is a key `Live::key` acts on and \
                                 NAMED_KEY_SPELLINGS does not say how the bindings \
                                 spell it"
                            )
                        });
                    assert!(
                        documented.contains(spelling),
                        "`{spelling}` (`NamedKey::{name}`) has no entry in the bindings"
                    );
                }
            }
        }

        // Floors, not counts. They are what `Live::key` actually holds today —
        // 32 characters, one range, two named keys — rather than a round number
        // under them, because a control surface is small enough that losing one
        // key is news and the scan going quiet is the thing being guarded
        // against. Three of them because they fail apart: a signature change
        // gives no arms at all, a broken literal reader gives named arms and no
        // characters, and a `NamedKey` renamed away gives characters and no
        // named ones. Raise them when a key is added; lowering one is a claim
        // that a control was deliberately removed.
        //
        // **It was 33 and is 32**, and that is the claim being made: ADR-0240
        // retired *Choose what the output shows*, so `v` is not a key any
        // more. The floor came down with the control rather than the control
        // being kept alive to hold a number up.
        assert!(
            chars >= 32,
            "only {chars} character keys read out of `Live::key` — the scan is not \
             seeing the match arms"
        );
        assert!(
            ranges >= 1,
            "no `'a'..='b'` arm read out of `Live::key` — slot focus is one"
        );
        assert!(
            named >= 2,
            "only {named} `NamedKey` arms read out of `Live::key` — escape and space \
             are two"
        );
    }

    /// **A character that occurs only in a binding's prose is not documented.**
    ///
    /// The reason [`documented_keys`] parses a column instead of asking
    /// `BINDINGS.contains(key)`, and it is not hypothetical: `?` appears inside
    /// the `, .` entry's description, so the substring form of this check would
    /// have called `?` documented while it was bound to nothing. That version
    /// would have looked stronger than the hard-coded array it replaced and
    /// enforced less.
    #[test]
    fn a_key_named_only_in_prose_is_not_documented() {
        // `?` twice in prose and never in the column: once in the description
        // beside a key, once on the continuation line under it. Both are places
        // the real text puts it, and each one is a different way the parse
        // could go wrong.
        let prose = concat!(
            "keys:\n",
            "  s          print the status line — the tracker writes ? there when\n",
            "             it is unsure, and ? again if it stays unsure\n",
        );
        // What a substring check sees, and it is a lie.
        assert!(prose.contains('?'));
        assert!(
            !documented_keys(prose).contains(&"?"),
            "a character in a description was counted as a documented key"
        );
        // The column, where a real binding lives, still reads.
        assert!(documented_keys(prose).contains(&"s"));
        // And in the real text `?` is now in the column, not only the prose.
        assert!(documented_keys(BINDINGS).contains(&"?"));
    }

    /// **Every method of [`Live`] that writes a record without going through
    /// [`Live::operate`]**, with the reason its operation cannot convert.
    ///
    /// This is the key handler's form of the single arm
    /// [`Live::run_surface`] keeps for `TapBeat`: a table rather than a
    /// comment, so a key that grows a second derivation of a record is a
    /// failing test naming the function instead of a line nobody reads. Every
    /// operation named here answers `Owed::NotSettled`, which is asserted
    /// separately — see
    /// [`the_keys_that_keep_their_own_path_are_the_ones_whose_record_is_not_settled`]
    /// — so an entry that stops being owed fails there rather than lingering
    /// here as a stale excuse.
    const OWED_RECORD_PATHS: &[(&str, &str)] = &[(
        "operate",
        "the route itself: this is where `written`'s records are written",
    )];

    /// The method a byte offset falls inside, read off the nearest `fn` above
    /// it at `impl` indentation.
    ///
    /// Four spaces and not any `fn `, because a closure or a nested helper
    /// would otherwise answer for the method it sits in. The needle is spelled
    /// with a leading newline so a `fn` inside an expression cannot match.
    fn enclosing_method(blanked: &str, at: usize) -> &str {
        let start = blanked[..at]
            .rfind("\n    fn ")
            .expect("every record written in this file is inside a method")
            + "\n    fn ".len();
        let name_end = blanked[start..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map_or(blanked.len(), |n| start + n);
        &blanked[start..name_end]
    }

    /// **A key reaches the deck through [`Live::operate`], or it is one of the
    /// owed ones and says why.**
    ///
    /// The route `karakuri-midi` took is *operation → `written` → record*, and
    /// the reason it could take it whole is that every operation a map line
    /// produces converts. A key handler cannot: three of its operations owe a
    /// record nobody can write yet, and routing one of those through `operate`
    /// would print the gap where the gesture used to be. It was seven, and
    /// four have left — `SetSync`, then `FadeDeck`, `Crossfade` and
    /// `SelectRenderer` together when the transition settings became a
    /// reading — each a row deleted here rather than kept, which is what the
    /// last loop below is for. So the ones that keep
    /// their own path are named here rather than left to be noticed, and this
    /// is what stops the list growing by accident — a record built beside the
    /// conversion is exactly the drift `karakuri-operation-record` exists to
    /// end, and `mix::gain_record`, `mix::opacity_record` and their neighbours
    /// went one at a time as each operation landed.
    ///
    /// Read out of the checked-in source, in the shape
    /// [`every_key_the_live_path_acts_on_is_documented`] set, with a floor so a
    /// scan that stops matching fails instead of passing everything.
    #[test]
    fn every_record_written_outside_operate_is_a_path_whose_conversion_is_owed() {
        let blanked = blank_comments_and_strings(SOURCE);
        let needle = concat!("self.", "record(");
        let mut reached: Vec<&str> = Vec::new();
        for (at, _) in blanked.match_indices(needle) {
            let name = enclosing_method(&blanked, at);
            assert!(
                OWED_RECORD_PATHS.iter().any(|(known, _)| *known == name),
                "`Live::{name}` writes a record without going through `Live::operate`, \
                 and OWED_RECORD_PATHS does not say why its operation cannot convert — \
                 a surface that derives a record beside the conversion is the drift \
                 `karakuri-operation-record` exists to end"
            );
            if !reached.contains(&name) {
                reached.push(name);
            }
        }
        // A floor rather than a count, and a low one: what is guarded against
        // is the scan going quiet, which would let every direct write through.
        // **One, where it was five, then four, then two.** `cycle_sync` was
        // the fifth; `fade_slot` and `cycle_renderer` were the third and
        // fourth and went together when the transition settings became a
        // reading; `wipe` was the second and went when the front shape joined
        // them. What is left is `operate` itself, which is the route rather
        // than a path around it — so this floor is now as low as it can go,
        // and the loop below is what actually keeps the table honest. A floor
        // above what is left would fail as a dead scan on the day a row was
        // correctly deleted — which is the one failure a guard against a dead
        // scan must not invent.
        assert!(
            !reached.is_empty(),
            "no method reads as a record writer — the scan is not seeing `Live`'s \
             bodies, and `Live::operate` itself is one: {reached:?}"
        );
        // And nothing in the table is a leftover. A path whose last direct
        // write moved to `operate` is a row to delete, not a permission to
        // keep.
        for (name, why) in OWED_RECORD_PATHS {
            assert!(
                reached.contains(name),
                "OWED_RECORD_PATHS excuses `Live::{name}` — {why} — and it writes no \
                 record of its own any more, so the row outlived what it was for"
            );
        }
    }

    /// **The keys that keep their own path are exactly the ones whose record
    /// is not settled**, and this is what will say so the day one changes.
    ///
    /// `b` and `, .` reach the beat tracker where they
    /// stand because `written` answers `Owed::NotSettled` for the operation
    /// each of them names. **`y`, `f g`, `x`, `r` and `c` are the five that
    /// have already gone**, and every one of them went the way this test
    /// names: `SetSync` stopped being owed when the conversion took a session
    /// tempo, `FadeDeck`, `Crossfade` and `SelectRenderer` stopped when it
    /// took the transition settings, and `Wipe` stopped when the front shape
    /// went over with them and the soft edge turned out to be the arriving
    /// deck's — so each key moved through [`Live::operate`]
    /// and its line here came out, taking `Live::fade_slot` and the last
    /// hand-built record with it. That is a statement about
    /// `karakuri-operation-record` rather than about this file, so it is
    /// checked against that crate: the day somebody settles one of these
    /// conversions, this fails and names the key that is now due to move
    /// through [`Live::operate`] — the promise `run_surface`'s `TapBeat` arm
    /// makes, kept by a test rather than by anyone remembering.
    #[test]
    fn the_keys_that_keep_their_own_path_are_the_ones_whose_record_is_not_settled() {
        use karakuri_operation_record::Owed;
        let owed = [
            ("b", Operation::TapBeat),
            (
                ", .",
                Operation::ScaleGrid {
                    by: karakuri_operation::GridScale::Double,
                },
            ),
        ];
        for (keys, operation) in owed {
            assert_eq!(
                karakuri_operation_record::written(&operation, &Current::default()),
                Written::Owed(Owed::NotSettled),
                "`{}` no longer owes its record, so `{keys}` reaches the deck by a \
                 derivation of its own where `Live::operate` would now do — see \
                 `Live::key`'s survey",
                operation.title()
            );
        }
    }

    /// **A save still being written when the run ends is waited for**, so the
    /// `save` record promised for every save that reached the disk
    /// (`docs/adr/0120-a-record-may-reach-outside-the-stream.md`)
    /// is in the stream.
    ///
    /// The window is small and it is exactly the one an operator is in: press
    /// `k`, read that it took, quit. The wait is bounded — see [`SAVE_WAIT`] —
    /// and the bound is what the second half of this checks: a save that never
    /// reports back costs the quit the bound and no more.
    #[test]
    fn a_save_in_flight_at_the_end_of_a_run_is_waited_for() {
        let (tx, rx) = std::sync::mpsc::channel();
        let late = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(120));
            tx.send(Saved {
                slot: 0,
                asked: Asked::Operator,
                id: "late".to_string(),
                outcome: Ok(()),
                // A hand pressed the key; nobody is waiting on a socket.
                reply: None,
            })
        });
        let landed = drained_saves(&rx, 1, Instant::now() + SAVE_WAIT);
        late.join().expect("the save thread").expect("it sent");
        assert_eq!(
            landed.len(),
            1,
            "the run quit before the save it was told to wait for reported back"
        );

        // **The bound, held against a save that never arrives.** The sender is
        // still alive, so there is nothing but the deadline to end this.
        let (_alive, rx) = std::sync::mpsc::channel::<Saved>();
        let started = Instant::now();
        let landed = drained_saves(&rx, 1, started + Duration::from_millis(80));
        assert!(landed.is_empty());
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "a save that never reports back held the quit past its deadline"
        );
    }

    /// **Both halves of the save path sit above everything in [`Live::frame`]
    /// that could stop them**, which is the whole of what makes an answer to a
    /// waiting client independent of there being a swapchain.
    ///
    /// [`Live::run_requests`] argues this for itself: a client asking to keep
    /// what is playing should not be waiting on a surface. The *outcome* drain
    /// used to sit below `frame::compose`, past three returns, so the argument
    /// was made and then half applied. On a window latched to
    /// `frame::Skip::Fault`, or returning `Outdated` every frame, the request
    /// was taken and the save thread wrote the file successfully — and the
    /// client waited out `mcp::SAVE_REPLY` to be told the outcome was neither
    /// success nor failure about a save already on disk, the terminal never
    /// said "saved as set X", and the `save` record promised for every save
    /// that reached the disk was withheld from the stream until the run quit.
    ///
    /// **This used to demand an early return and assert the calls were above
    /// it.** There is no longer one: a sink with no target stopped being a
    /// reason to leave the function when a frame stopped belonging to one sink
    /// — see `frame`'s module documentation — so a frame that publishes nowhere
    /// now runs to the end. Demanding a `return` would make this test fail for
    /// the reason the defect it guards was fixed, which is the wrong question.
    /// What it asks instead is the property that outlives either shape: the two
    /// calls come above `frame::compose`, which is the GPU work and everything
    /// that has ever been a reason to bail out, **and** above the first
    /// `return` if one ever comes back. The second half is dormant today and is
    /// the half that matters on the day it is not.
    ///
    /// **Read off the source, and that is the honest description of what this
    /// can reach.** `Live::frame` needs a window, a GPU and an event loop, and
    /// the two replay defects this project found both lived in this one
    /// function's statement order
    /// (`docs/adr/0078-a-frame-that-is-discarded-must-not-already-have-been-recorded.md`).
    /// The defect here is a statement order too, and this asserts it where it
    /// is, rather than asserting nothing and calling it untestable. Comment
    /// lines are dropped first, so prose about returning cannot stand in for a
    /// `return`.
    #[test]
    fn a_frame_attends_to_its_saves_before_anything_can_stop_them() {
        let source = include_str!("main.rs");
        let body = source
            .split_once("\n    fn frame(&mut self) {")
            .expect("`Live::frame` is no longer spelled that way")
            .1;
        let body = body
            .split("\n    fn ")
            .next()
            .expect("the end of `Live::frame`");
        let code: Vec<&str> = body
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        let code = code.join("\n");

        // The end of the function if it never returns early, which is what it
        // does today — every position below is genuinely above it either way.
        let first_return = code.find("return").unwrap_or(code.len());
        let composed = code
            .find("frame::compose(")
            .expect("`Live::frame` no longer composes a frame, and this test is about where");
        for call in ["self.run_requests();", "self.finished_saves();"] {
            let at = code
                .find(call)
                .unwrap_or_else(|| panic!("`Live::frame` no longer calls `{call}`"));
            assert!(
                at < composed,
                "`{call}` is below `frame::compose` in `Live::frame`: a save's outcome has \
                 nothing to do with whether there is a surface to draw on"
            );
            assert!(
                at < first_return,
                "`{call}` is below an early return in `Live::frame`: a frame that publishes \
                 nowhere takes saves and never answers for them"
            );
        }
    }

    // The eight that build a Set to save from. A live save is read out of a running
    // deck, and there is no deck without a device — the eleven above test the record
    // and the rollback bookkeeping around that, and take none.
    // See `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs`.
    mod gpu {
        use super::*;

        /// **One fold, so the card and the uniform cannot disagree.**
        ///
        /// The number in a `param_decl` and the number the engine loads into a
        /// node's uniform are the same declaration read twice, and until this
        /// commit there was exactly one reader of it — private to
        /// `karakuri-engine`, with a note saying a second evaluator elsewhere would
        /// agree with the shader by coincidence. The metadata writer is that
        /// elsewhere. So the fold moved to `karakuri_ir::Param::default_scalar` and
        /// both call it, and this is what fails if either grows a reader of its own:
        /// a card claiming `0.0` where the run loaded `-0.35` describes a procedure
        /// nobody ran, and nothing downstream could say which of the two was wrong.
        ///
        /// **Both directions.** Every default the card states must be the value the
        /// built Set is running, *and* every value the Set is running must be
        /// stated — one of those alone passes when a writer silently drops the
        /// declaration it cannot fold.
        ///
        /// The Set is built with no overrides and no Set file, so what a node holds
        /// is exactly what its `.kir` declared.
        #[test]
        fn the_engine_and_the_metadata_writer_cannot_disagree_about_a_default() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let paths = vec![
                kir(&dir, "a.kir", &signed_l1()),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            let (material, placed) = slot(&paths);
            let set = set_of(&gpu, &material, &[16384], &[3], &[], None);
            let store = karakuri_store::store::Store::open(dir.path().join("store"))
                .expect("a store opens");
            let hash = placed[0].put(&store).expect("the artifact is stored");

            let mut on_the_card: Vec<(String, f32)> = store
                .read_meta(&hash)
                .expect("a card beside the artifact")
                .iter()
                .filter_map(|l| match l.record() {
                    Record::ParamDecl {
                        key,
                        default: Some(v),
                        ..
                    } => Some((key.clone(), *v)),
                    _ => None,
                })
                .collect();
            let mut in_the_uniform: Vec<(String, f32)> = set
                .params()
                .filter(|(layer, index, ..)| *layer == karakuri_ir::Kind::L1 && *index == 0)
                .map(|(_, _, key, value)| (key.to_string(), value))
                .collect();
            on_the_card.sort_by(|a, b| a.0.cmp(&b.0));
            in_the_uniform.sort_by(|a, b| a.0.cmp(&b.0));

            assert_eq!(
                on_the_card, in_the_uniform,
                "the card and the uniform read the same declaration and came back with \
             different numbers, which means there are two folds again"
            );
            // Not a vacuous agreement: both have to have folded the negation. Two
            // readers that both dropped it would be equal and both wrong.
            assert!(
                on_the_card.contains(&("signed".to_string(), -0.35)),
                "neither reader folded the negation, so they agree about nothing: \
             {on_the_card:?}"
            );
        }
        /// **A live save writes a file that loads back into the same material** —
        /// and does it for a Set of *two* geometries, which is where the numbers
        /// stop being interchangeable.
        ///
        /// Two geometries at two different capacities and two different salts,
        /// because that is the shape a single number cannot describe: a saver
        /// reaching for `Set::capacity` gets the sum, which is neither geometry's,
        /// and the file it writes is refused for having one capacity where the Set
        /// has two. `Set::source_capacities` is the reading that is per geometry,
        /// which is what the record is.
        #[test]
        fn a_live_save_reads_back_into_the_material_it_was_taken_from() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let l1 = example("drift_shell.kir");
            let paths = vec![
                kir(&dir, "a.kir", &l1),
                kir(
                    &dir,
                    "b.kir",
                    &l1.replace("proc drift_shell", "proc drift_two"),
                ),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            let (material, placed) = slot(&paths);
            // Different from each other and from the declared default, so that a
            // file which recorded either the sum or the declaration is visibly
            // wrong rather than accidentally right.
            let capacities = [8192, 16384];
            let salts = [11, 22];
            let set = set_of(&gpu, &material, &capacities, &salts, &[], None);

            let root = dir.path().join("store");
            let running = launched(&placed);
            let loaded = save_and_load(
                &root,
                "live",
                &set,
                live_sources(running.playing(0), &placed),
            );

            assert_eq!(
                loaded
                    .l1s
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
                ["drift_shell", "drift_two"],
                "the geometries came back as something else"
            );
            assert_eq!(
                loaded
                    .l4s
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
                ["soft_points"]
            );
            assert_eq!(
                loaded.capacities,
                vec![Some(8192), Some(16384)],
                "each geometry's own capacity is what the file has to carry"
            );
            assert_eq!(
                loaded.salts,
                vec![Some(11), Some(22)],
                "each geometry's own salt is what the file has to carry"
            );
        }
        /// **A save after a rebuild records the camera the slot was loaded with**,
        /// and not the built-in orbit's defaults.
        ///
        /// This is the whole path and deliberately not a piece of it: a slot aimed
        /// by a `camera` record, the watcher a `--watch` run gives it, an edit to a
        /// file, the build worker, the swap — and then the same `playing_values`
        /// the `k` key reads through. Every link in it was correct on its own while
        /// the chain silently re-aimed the slot, because the one that was missing
        /// was the request in the middle: `Set::build_many` starts every Set from
        /// `Orbit::default()`, so the rebuilt Set was aimed at the defaults and the
        /// saver recorded exactly what it found. That is why the loss stopped being
        /// a wrong picture and became a file — the operator's next preset was
        /// written with a camera nobody had chosen.
        ///
        /// **Driven through `HotSwap` rather than by calling the worker's code**,
        /// for the reason [`save_and_load`] goes through [`Save::run`]: a rebuild
        /// assembled by hand here would be a second copy of the rebuild, and a test
        /// of a copy is a test of nothing. `begin_frame` is the only place a build
        /// is installed, and it is enough on its own — nothing here has to render.
        #[test]
        fn a_save_after_a_rebuild_records_the_camera_the_slot_was_loaded_with() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let l1 = example("drift_shell.kir");
            let paths = vec![
                kir(&dir, "a.kir", &l1),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            let (material, _placed) = slot(&paths);
            // Six numbers no default produces. A `camera` record spells two of them
            // and the loader fills the rest from `Orbit::default()`, so a Set file
            // cannot actually deliver these four — they are here because what is
            // under test is the *carrying*, and a value that differs in every field
            // says which fields were carried.
            let aimed = karakuri_engine::camera::Orbit {
                radius: 3.25,
                speed: 0.75,
                height: -1.5,
                fov_y: 0.9,
                near: 0.25,
                far: 250.0,
            };
            let capacity = 8192;
            let salts = [11];
            let set = set_of(&gpu, &material, &[capacity], &salts, &[], Some(aimed));

            // The watcher `build_deck` gives the slot, stating what the slot is
            // running at — the camera among it, from the one reading the Set above
            // was built with.
            let watcher = watch::Watch::new(
                0,
                Named::bare(paths[0].clone()),
                vec![Named::bare(paths[1].clone())],
                karakuri_engine::set::Layering::Overdraw,
                None,
                Some(capacity),
                salts[0],
                salts.to_vec(),
                aimed,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            );
            let mut swap = HotSwap::new(
                &gpu.device,
                &gpu.queue,
                set,
                DEFAULT_BUDGET_MS,
                Box::new(watcher),
            );

            // The edit an operator makes, in the one form that changes nothing
            // about the material: the watcher compares contents, so a comment is
            // enough to make this a save it wakes on — and it keeps the rebuilt Set
            // the same Set, so the only thing that can differ is what was carried.
            std::fs::write(&paths[0], format!("{l1}\n// an edit\n")).expect("edit the geometry");

            // Bounded, and generous: this covers two poll intervals of debouncing,
            // four compiler stages, two shader modules and a whole-capacity upload,
            // on whatever machine is running the suite.
            let deadline = Instant::now() + Duration::from_secs(30);
            let mut seen: Vec<String> = Vec::new();
            loop {
                swap.begin_frame(&gpu.device);
                let mut swapped = false;
                for event in swap.events() {
                    swapped |= matches!(event, Event::Swapped { .. });
                    seen.push(event.to_string());
                }
                // **Read at the swap and not a frame later.** The outgoing Set is
                // aimed correctly too, so a rollback would put the right camera
                // back and hide exactly the defect this is about.
                if swapped {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "the edit never rebuilt; saw {seen:?}"
                );
                std::thread::sleep(Duration::from_millis(5));
            }

            let recorded = playing_values(swap.set(), &[]).camera;
            let six = |o: &karakuri_engine::camera::Orbit| {
                (o.radius, o.speed, o.height, o.fov_y, o.near, o.far)
            };
            assert_eq!(
                six(&recorded),
                six(&aimed),
                "the save after a rebuild would have written a camera the operator never aimed"
            );
        }
        /// **A save after a parameter moved records the moved value.**
        ///
        /// The Set is built with `radius` at 1.0, which is what a `--param
        /// radius=1.0` would have put there, and then moved to 2.6 the way a record
        /// moves it mid-run. The file has to say 2.6: a writer holding its own copy
        /// of what the run was started with records a number nobody has seen, which
        /// is the failure `saving_capacities` was fixed over, arriving one surface
        /// further along.
        ///
        /// Addressed rather than wildcard, because that is what the Set holds — a
        /// value per node — and because it is the only form that can be checked
        /// against the node that has it.
        #[test]
        fn a_live_save_records_a_param_where_the_run_moved_it_to() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let paths = vec![
                kir(&dir, "a.kir", &example("drift_shell.kir")),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            let (material, placed) = slot(&paths);
            let flag = ParamWrite::everywhere("radius", 1.0);
            let mut set = set_of(
                &gpu,
                &material,
                &[4096],
                &[7],
                std::slice::from_ref(&flag),
                None,
            );
            assert_eq!(
                set.param("radius"),
                Some(1.0),
                "the run started at the flag"
            );

            // What a `param` record does mid-set, through the one entry point both
            // a record and a key press come through.
            set.write_param(&ParamWrite::everywhere("radius", 2.6))
                .expect("every node of a Set nobody has spoken for is manual");

            let root = dir.path().join("store");
            let running = launched(&placed);
            let loaded = save_and_load(
                &root,
                "moved",
                &set,
                live_sources(running.playing(0), &placed),
            );

            let radius: Vec<&ParamWrite> =
                loaded.params.iter().filter(|p| p.key == "radius").collect();
            assert_eq!(
                radius.len(),
                1,
                "one geometry declares `radius`, so one line records it: {radius:?}"
            );
            assert_eq!(
                radius[0].value, 2.6,
                "the file recorded the value the run was started with, not the one it \
             was playing"
            );
            assert_eq!(
                radius[0].at,
                Some((karakuri_ir::Kind::L1, 0)),
                "a param is recorded against the node that declares it"
            );
        }
        /// **A save after a rollback onto an earlier rebuild records that
        /// rebuild**, not the version left on disk and not the one it launched with.
        ///
        /// This is the whole reason the writer stopped reading files. A build that
        /// compiles and is then refused for cost leaves a newer `.kir` sitting on
        /// disk under the same path — so the path and the picture disagree, and they
        /// disagree in exactly the situation where an operator most wants to keep
        /// what they can see.
        ///
        /// **Three versions, all distinguishable**, which is what makes the
        /// assertion about the answer that was chosen rather than about a file that
        /// could only ever have said one thing. The slot launches on `drift_shell`,
        /// a rebuild to `drift_two` lands, a rebuild to `refused_edit` lands and is
        /// then rolled back — so the picture is `drift_two`, the disk is
        /// `refused_edit`, and the launch version is neither. The control is the
        /// save taken before any of it, which the same slot answers `drift_shell`.
        ///
        /// This test used to build what the slot was playing by hand, which is why
        /// it only ever reached the rollback that lands on an *earlier rebuild*. The
        /// first rollback of a slot is the case below, and the transition is driven
        /// through [`Running`] here so that the two are the same machinery.
        #[test]
        fn a_save_after_a_rollback_records_what_is_on_screen() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let at_start = example("drift_shell.kir");
            let renderer = example("soft_points.kir");
            let paths = vec![kir(&dir, "a.kir", &at_start), kir(&dir, "r.kir", &renderer)];
            let (material, placed) = slot(&paths);
            let set = set_of(&gpu, &material, &[4096], &[7], &[], None);
            let root = dir.path().join("store");

            let mut running = launched(&placed);

            // **The control**, taken before anything has rebuilt: the slot answers
            // with what it launched on. Without it, the assertion below would be
            // about a file that had only one version to name.
            let at_launch = save_and_load(
                &root,
                "at_launch",
                &set,
                live_sources(running.playing(0), &placed),
            );
            assert_eq!(
                at_launch.l1s[0].name, "drift_shell",
                "a slot nothing has rebuilt runs what it launched with"
            );

            // What the watcher does on the worker thread when a build lands: the
            // sources it compiled go into the store, and their hashes are what the
            // slot is recorded as running.
            let store = karakuri_store::store::Store::open(&root).expect("store");
            let on_screen = at_start.replace("proc drift_shell", "proc drift_two");
            std::fs::write(&paths[0], &on_screen).expect("the edit that stayed");
            running.landed(
                0,
                Some(vec![
                    stored(&store, "L1", &on_screen),
                    stored(&store, "L4", &renderer),
                ]),
            );

            // The edit that compiled and was then refused for cost. The file on
            // disk is this from here on; the picture goes back to the one above.
            let refused = at_start.replace("proc drift_shell", "proc refused_edit");
            std::fs::write(&paths[0], &refused).expect("the rolled-back edit");
            running.landed(
                0,
                Some(vec![
                    stored(&store, "L1", &refused),
                    stored(&store, "L4", &renderer),
                ]),
            );
            running.rolled_back(0);

            let kept = save_and_load(
                &root,
                "kept",
                &set,
                live_sources(running.playing(0), &placed),
            );
            assert_eq!(
                kept.l1s[0].name, "drift_two",
                "the save wrote down something other than the rebuild the slot fell \
             back onto"
            );
            assert!(
                std::fs::read_to_string(&paths[0])
                    .expect("the file")
                    .contains("proc refused_edit"),
                "the path still has to hold the refused version, or this test is \
             checking nothing"
            );
        }
        /// **The first rollback of a slot saves what is on screen.**
        ///
        /// The case a rollback onto an earlier rebuild cannot reach, and the one
        /// that had no hash anywhere pointing at it. Nothing has swapped into this
        /// slot before the build that is refused, so what comes back is the version
        /// the run launched on — and until those bytes were put in the store at
        /// launch, the saver answered by reading the path, which by then holds the
        /// refused version.
        ///
        /// **It fails in the worst direction**, which is why this is the test the
        /// fix is measured by: the operator is told the save took, and the file it
        /// wrote names a version that was never on screen.
        #[test]
        fn the_first_rollback_of_a_slot_saves_what_is_on_screen() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let at_start = example("drift_shell.kir");
            let renderer = example("soft_points.kir");
            let paths = vec![kir(&dir, "a.kir", &at_start), kir(&dir, "r.kir", &renderer)];
            let (material, placed) = slot(&paths);
            let set = set_of(&gpu, &material, &[4096], &[7], &[], None);
            let root = dir.path().join("store");

            let mut running = launched(&placed);

            // The operator edits, it compiles, the watcher stores it and the swap
            // lands. The file on disk is the edit from here on.
            let refused = at_start.replace("proc drift_shell", "proc refused_edit");
            std::fs::write(&paths[0], &refused).expect("the edit");
            let store = karakuri_store::store::Store::open(&root).expect("store");
            running.landed(
                0,
                Some(vec![
                    stored(&store, "L1", &refused),
                    stored(&store, "L4", &renderer),
                ]),
            );
            // The governor refuses it. **The first rollback of this slot**: nothing
            // swapped in before the build that just went out.
            running.rolled_back(0);

            let kept = save_and_load(
                &root,
                "kept",
                &set,
                live_sources(running.playing(0), &placed),
            );
            assert_eq!(
                kept.l1s[0].name, "drift_shell",
                "the save wrote down the version the governor refused — the one left \
             on disk, which never reached the screen"
            );
        }
        /// **A run with no watcher saves the version it is still drawing**, however
        /// far the file underneath it has moved.
        ///
        /// Nothing picks a `.kir` up without `--watch`: an editor writing over the
        /// path, an `--mcp` write with no watcher behind it — which the program's
        /// `mcp.rs` says
        /// out loud when it takes one — and an edit that failed to compile all leave
        /// the same state, and it is a state that never resolves on its own. The
        /// disk moved and the picture did not.
        ///
        /// **This is why the launch addresses are seeded for every windowed run and
        /// not only an editable one.** There is no watcher here to hash anything
        /// later, so a run gated on `editable()` would have nothing to save from
        /// and would go on answering with the path. The bytes behind those
        /// addresses reach the store here, at the save — see
        /// [`a_windowed_run_creates_no_store_until_something_is_saved`] for the
        /// other half of that, which is that nothing reaches it before.
        #[test]
        fn a_run_with_no_watcher_saves_the_version_it_is_still_drawing() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let at_start = example("drift_shell.kir");
            let paths = vec![
                kir(&dir, "a.kir", &at_start),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            let (material, placed) = slot(&paths);
            let set = set_of(&gpu, &material, &[4096], &[7], &[], None);
            let root = dir.path().join("store");

            let running = launched(&placed);

            // Something else rewrote the file. Nothing in this run is watching it,
            // so no build is requested, nothing lands, and the deck goes on drawing
            // what it built at startup for as long as the run lasts.
            std::fs::write(
                &paths[0],
                at_start.replace("proc drift_shell", "proc edited_outside"),
            )
            .expect("the edit nothing picked up");

            let kept = save_and_load(
                &root,
                "kept",
                &set,
                live_sources(running.playing(0), &placed),
            );
            assert_eq!(
                kept.l1s[0].name, "drift_shell",
                "the save read the path and wrote down an edit this run never \
             compiled, let alone drew"
            );
        }
        /// **A `.kir` rewritten between the compile and the first frame cannot
        /// reach a save.**
        ///
        /// The window is real and it is not short: between `sort_slot` and the
        /// first frame sit the adapter request, the deck build, `measure_slots`,
        /// and the audio, MIDI, tempo and **MCP server** starts. The launch seeding
        /// used to `std::fs::read` each path again at the end of that, so anything
        /// rewriting a file in between moved the slot's address onto bytes the deck
        /// had never compiled. If the rewrite did not compile, no watcher ever
        /// corrected it — `k` then wrote a Set naming a procedure that had never
        /// been on screen, and the file did not load back at all.
        ///
        /// **Distinct from
        /// [`a_run_with_no_watcher_saves_the_version_it_is_still_drawing`]**, which
        /// rewrites the file after the run is under way. This one rewrites it
        /// inside the startup sequence, which is the window a second read opens and
        /// carrying the bytes closes.
        ///
        /// Two assertions, the first crisp and the second end to end: the address
        /// the slot reports, and the file that comes back off the disk.
        #[test]
        fn a_rewrite_between_the_compile_and_the_first_frame_cannot_reach_a_save() {
            let gpu = Gpu::headless().expect("no GPU");
            let dir = tempfile::tempdir().expect("tempdir");
            let at_start = example("drift_shell.kir");
            let paths = vec![
                kir(&dir, "a.kir", &at_start),
                kir(&dir, "r.kir", &example("soft_points.kir")),
            ];
            // The compile. Everything the run says about these nodes from here on
            // is a function of the bytes this read.
            let (material, placed) = slot(&paths);
            let set = set_of(&gpu, &material, &[4096], &[7], &[], None);

            // Startup is not finished. Something rewrites the file — a formatter on
            // save, an editor, a model over MCP that connected as the server came
            // up — and this one does not compile, so nothing will ever correct it.
            std::fs::write(
                &paths[0],
                at_start.replace("proc drift_shell", "proc rewritten_between {{{"),
            )
            .expect("the rewrite inside the startup sequence");

            let root = dir.path().join("store");
            let running = launched(&placed);
            let sources = live_sources(running.playing(0), &placed);
            assert_eq!(
                sources.0[0].hash,
                karakuri_store::hash::Hash::of(at_start.as_bytes()),
                "the slot is addressed by bytes this run never compiled, so what it \
             saves is a version that was never on screen"
            );

            let kept = save_and_load(&root, "kept", &set, sources);
            assert_eq!(
                kept.l1s[0].name, "drift_shell",
                "the save wrote down the rewrite rather than the material the deck \
             was built from"
            );
        }
    }
}

/// **What `wire_input` asks the render loop for, and what the loop does with
/// it.**
///
/// The three points [`mcp::WireRequest`] states are the three these defend:
/// replace keyed on the input, re-aim the slot so it rebuilds, and answer once
/// at the frame it was applied on. Everything below [`rewired`] needs is a list
/// of edges and a channel — no window, no GPU and no `Deck` — which is why that
/// function is not a method; the one test here that goes through the socket is
/// the one about the *answer*, because only a real client can be told anything.
#[cfg(test)]
mod wire_tests {
    use super::*;
    use std::io::{BufRead, Write};

    fn edge(node: &str, slot: &str, to: &str) -> karakuri_engine::set::Edge {
        karakuri_engine::set::Edge {
            node: node.to_string(),
            slot: slot.to_string(),
            to: to.to_string(),
        }
    }

    /// A watcher's worth of aim, with nothing in it that matters here except
    /// the edges: what these tests read off a re-point is the wiring it carries
    /// and whether one arrived at all.
    fn aimed(edges: Vec<karakuri_engine::set::Edge>) -> watch::Aim {
        watch::Aim {
            head: Named::bare("head.kir"),
            rest: Vec::new(),
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            capacity: None,
            seed_salt: 7,
            salts: vec![7],
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges,
            authorities: Vec::new(),
        }
    }

    /// One watched slot, and the end a re-point arrives on.
    fn watcher(
        edges: Vec<karakuri_engine::set::Edge>,
    ) -> (Aiming, std::sync::mpsc::Receiver<watch::Aim>) {
        let (aim, aimed_at) = std::sync::mpsc::channel();
        (
            Aiming {
                aim,
                at: aimed(edges),
            },
            aimed_at,
        )
    }

    /// **An edge is replaced on the input it binds, and no other edge moves.**
    ///
    /// The replacement is keyed on `(node, slot)` — the node that declares the
    /// input and what its procedure calls it — so a second `uses` on the *same
    /// node* is a different entry, and so is the same input name on a different
    /// node. A key that were only the node would silently unbind `morph.near`
    /// when `morph.far` was rewired; one that were only the input name would do
    /// it across nodes. Both are here, because both would build a Set nobody
    /// asked for and neither shows up on the call that did it.
    #[test]
    fn a_wire_replaces_the_edge_on_the_input_it_binds_and_leaves_every_other_alone() {
        let mut edges = vec![
            edge("morph", "far", "sphere_shell"),
            edge("morph", "near", "lattice"),
            edge("veil", "far", "torus"),
        ];
        let (aiming, aims) = watcher(edges.clone());
        let mut slots = vec![Some(aiming)];

        let said = rewired(
            &[(0, edge("morph", "far", "drift_shell"))],
            &mut edges,
            &mut slots,
            1,
        );

        assert_eq!(said.len(), 1);
        assert!(said[0].is_ok(), "{:?}", said[0]);
        assert_eq!(
            edges.len(),
            3,
            "the run is wired with {} edges after replacing one of three: {edges:?}",
            edges.len()
        );
        assert!(
            edges.contains(&edge("morph", "far", "drift_shell")),
            "the edge asked for is not in the run's wiring: {edges:?}"
        );
        assert!(
            edges.contains(&edge("morph", "near", "lattice")),
            "rewiring `morph.far` took `morph.near` with it: {edges:?}"
        );
        assert!(
            edges.contains(&edge("veil", "far", "torus")),
            "rewiring `morph.far` took `veil.far` with it — the key is the node and the \
             input, not the input alone: {edges:?}"
        );
        // And what the watcher was handed is the same list, not the one it
        // started with: a rebuild restates its own edges, so a re-aim that
        // carried the old wiring would put the old edge back on the next build.
        let sent = aims.try_recv().expect("the slot was not re-aimed");
        assert_eq!(sent.edges, edges, "the re-aim carried a different wiring");
    }

    /// **A second wire on one input replaces rather than appends.**
    ///
    /// `SetError::SlotBoundTwice` refuses two edges on one input where the Set
    /// is built, so an append would make the *second* call on an input a
    /// refusal — a model that changed its mind would have wired the slot into a
    /// state it cannot build and cannot leave. The two calls are separate
    /// frames here, which is the ordinary case; the same thing on one frame is
    /// the test below.
    #[test]
    fn a_second_wire_on_one_input_replaces_rather_than_appends() {
        let mut edges = vec![edge("morph", "far", "sphere_shell")];
        let (aiming, aims) = watcher(edges.clone());
        let mut slots = vec![Some(aiming)];

        for to in ["drift_shell", "lattice_shell"] {
            let said = rewired(&[(0, edge("morph", "far", to))], &mut edges, &mut slots, 1);
            assert!(said[0].is_ok(), "{:?}", said[0]);
        }

        assert_eq!(
            edges,
            vec![edge("morph", "far", "lattice_shell")],
            "one input is bound {} times: a Set built from this is refused for \
             `SlotBoundTwice` and the second call is where a model loses its way back",
            edges.len()
        );
        // Both frames re-aimed the slot, and the second one carries the second
        // edge — the first would leave the picture on the wiring the model
        // changed its mind about.
        let sent: Vec<watch::Aim> = aims.try_iter().collect();
        assert_eq!(
            sent.len(),
            2,
            "one frame's rewiring did not reach the watcher"
        );
        assert_eq!(sent[1].edges, vec![edge("morph", "far", "lattice_shell")]);
    }

    /// **Two wires on one input on one frame: both are applied in order, the
    /// later one is what the run holds, and the earlier one is told so.**
    ///
    /// One aim goes out per slot after every edge on the frame is in the list,
    /// so the rebuild carries the wiring the frame ended with rather than an
    /// intermediate one. And the reply to the overwritten request says it was
    /// overwritten: it did write its edge, so a refusal would be false, and a
    /// bare "wired" would be a true sentence about a state the run no longer
    /// held by the end of the frame it was sent on.
    #[test]
    fn two_wires_on_one_input_in_one_frame_leave_the_run_wired_with_the_later_one() {
        let mut edges = vec![edge("morph", "far", "sphere_shell")];
        let (aiming, aims) = watcher(edges.clone());
        let mut slots = vec![Some(aiming)];

        let said = rewired(
            &[
                (0, edge("morph", "far", "drift_shell")),
                (0, edge("morph", "far", "lattice_shell")),
            ],
            &mut edges,
            &mut slots,
            1,
        );

        assert_eq!(
            edges,
            vec![edge("morph", "far", "lattice_shell")],
            "the run is wired with something other than the last edge of the frame"
        );
        let first = said[0].as_ref().expect("the first was applied");
        assert!(
            first.contains("replaced it with `lattice_shell`"),
            "the overwritten request was told its edge stands: {first}"
        );
        assert!(
            said[1]
                .as_ref()
                .expect("the second was applied")
                .contains("recompiling"),
            "the edge the run kept was not reported as rebuilding: {:?}",
            said[1]
        );
        let sent: Vec<watch::Aim> = aims.try_iter().collect();
        assert_eq!(
            sent.len(),
            1,
            "two edges on one frame re-aimed the slot {} times: a frame rebuilds once, at \
             the wiring it ended with",
            sent.len()
        );
        assert_eq!(sent[0].edges, vec![edge("morph", "far", "lattice_shell")]);
    }

    /// **A slot this deck does not hold is refused, and nothing is rewired.**
    ///
    /// The MCP server checks the number against its `Slots` before it sends, so
    /// a model meets the refusal there — this is the guard that does not depend
    /// on the surface that asked having one, which is exactly what
    /// `Live::save_set` says about the same check. The run's wiring is the part
    /// worth asserting: a refusal that had already edited the list would leave
    /// the deck wired by a call it said it had refused.
    #[test]
    fn a_wire_naming_a_slot_this_deck_does_not_hold_is_refused_and_nothing_is_rewired() {
        let mut edges = vec![edge("morph", "far", "sphere_shell")];
        let (aiming, aims) = watcher(edges.clone());
        let mut slots = vec![Some(aiming)];

        // With one that *is* applied in front of it, on the same input: a
        // refusal writes nothing, so the request before it must not be told
        // that a later one replaced its edge.
        let said = rewired(
            &[
                (0, edge("morph", "far", "lattice_shell")),
                (4, edge("morph", "far", "drift_shell")),
            ],
            &mut edges,
            &mut slots,
            1,
        );

        let refusal = said[1]
            .as_ref()
            .expect_err("a slot 4 of a one-slot deck was accepted");
        assert!(refusal.contains("no slot 4"), "{refusal}");
        assert!(refusal.contains("nothing was rewired"), "{refusal}");
        assert_eq!(
            edges,
            vec![edge("morph", "far", "lattice_shell")],
            "a refused request edited the run's wiring anyway"
        );
        let kept = said[0].as_ref().expect("the slot 0 request was applied");
        assert!(
            !kept.contains("replaced it with"),
            "a request that was refused was reported as having replaced the edge in front \
             of it: {kept}"
        );
        let sent: Vec<watch::Aim> = aims.try_iter().collect();
        assert_eq!(
            sent.len(),
            1,
            "a refused request re-aimed a watcher, so a slot is rebuilding for a call that \
             was told nothing happened"
        );
        assert_eq!(sent[0].edges, vec![edge("morph", "far", "lattice_shell")]);
    }

    /// **A slot with no watcher is not a failure, and the reply says what did
    /// not happen.**
    ///
    /// A run without `--watch` has nothing that rebuilds. The edge is still the
    /// run's — a `save_set` records it — so a refusal would be false; and a
    /// bare "wired" would let a model wait for a picture that is never going to
    /// change. `mcp::wire_input` adds the same fact from its side, where it is
    /// the only thing that knows how the run was started.
    #[test]
    fn a_wire_on_a_slot_with_no_watcher_is_applied_and_says_nothing_rebuilds() {
        let mut edges = Vec::new();
        let mut slots: Vec<Option<Aiming>> = vec![None];

        let said = rewired(
            &[(0, edge("morph", "far", "drift_shell"))],
            &mut edges,
            &mut slots,
            1,
        );

        let line = said[0]
            .as_ref()
            .expect("a run without a watcher refused an edge");
        assert!(
            line.contains("no watcher") && line.contains("save_set"),
            "{line}"
        );
        assert_eq!(edges, vec![edge("morph", "far", "drift_shell")]);
    }

    /// **The frame drains the edges, and not only the saves.**
    ///
    /// This is the failure the whole surface was in when `wire_input` landed:
    /// the tool was finished, the channel was there, and the render loop
    /// answered none of it — so every call waited out `WIRE_REPLY` and came
    /// back with a true sentence saying nothing had been rewired. Everything
    /// else here tests what [`rewired`] decides; nothing else tests that a
    /// frame ever asks it. `Live::run_requests` needs a window and a GPU, so
    /// this is read off the source, which is
    /// `a_frame_attends_to_its_saves_before_anything_can_stop_them`'s own
    /// technique and for its reason: asserting where a statement is beats
    /// asserting nothing and calling it untestable. Comment lines are dropped
    /// first, so prose about draining cannot stand in for a drain.
    #[test]
    fn a_frame_takes_the_edges_a_client_asked_for_and_not_only_the_saves() {
        let source = include_str!("main.rs");
        let body = source
            .split_once("\n    fn run_requests(&mut self) {")
            .expect("`Live::run_requests` is no longer spelled that way")
            .1;
        let body = body
            .split("\n    }")
            .next()
            .expect("the end of `Live::run_requests`");
        let code: Vec<&str> = body
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        let code = code.join("\n");
        for call in ["mcp.wires()", "self.rewire(wires)"] {
            assert!(
                code.contains(call),
                "`Live::run_requests` does not `{call}`: a `wire_input` call on this run \
                 waits out its deadline and is told the loop never took the edge"
            );
        }
    }

    // -- over the socket -------------------------------------------------

    /// One tool call, over TCP exactly as a client makes it. The answer is the
    /// thing being tested, so nothing here shares a channel with the server —
    /// it is the wire.
    fn call(port: u16, name: &str, args: serde_json::Value) -> (bool, String) {
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": name, "arguments": args},
        })
        .to_string();
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .expect("read timeout");
        stream
            .write_all(
                format!(
                    "POST / HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .expect("write");
        let mut reader = std::io::BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).expect("status line");
        let mut length = 0usize;
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).expect("header");
            let header = header.trim_end();
            if header.is_empty() {
                break;
            }
            if let Some((name, value)) = header.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse().unwrap_or(0);
                }
            }
        }
        let mut body = vec![0u8; length];
        std::io::Read::read_exact(&mut reader, &mut body).expect("body");
        let reply: serde_json::Value =
            serde_json::from_slice(&body).expect("the server answered something that is not JSON");
        let result = &reply["result"];
        (
            result["isError"].as_bool().unwrap_or(true),
            result["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        )
    }

    /// **A `wire_input` call is answered at the frame the edge was applied on,
    /// and the run is wired with it.**
    ///
    /// End to end: the call goes over the socket, the request crosses
    /// `mcp::Reporter::wires`, a stand-in frame loop applies it with the same
    /// [`rewired`] the real one calls, and the answer comes back down the
    /// connection the model is holding open. **Without the drain this is
    /// exactly the failure that was here**: the tool waits out `WIRE_REPLY` and
    /// says *the render loop had not taken this edge*, which is true, is loud,
    /// and is not a rewiring.
    ///
    /// The frame loop is a thread rather than a `Live` because a `Live` needs a
    /// window and a GPU. What it stands in for is the drain and the answer, and
    /// those are the whole of `Live::rewire`.
    #[test]
    fn a_wire_request_is_answered_at_the_frame_it_was_applied_on() {
        let dir = tempfile::tempdir().expect("tempdir");
        let l1 = dir.path().join("l1.kir");
        std::fs::write(&l1, "proc probe { kind L1 }").expect("fixture");
        let reporter = mcp::serve(
            0,
            mcp::Slots(vec![(l1, Vec::new())]),
            dir.path().join("store"),
            true,
            karakuri_environment::Opening::closed(),
        )
        .expect("serve");
        let port = reporter.port();

        // What the run is wired with, shared so the test can read it back —
        // the render loop's own copy is `Live::edges` and nothing else holds
        // one.
        let edges = Arc::new(std::sync::Mutex::new(vec![edge(
            "morph",
            "far",
            "sphere_shell",
        )]));
        let (aiming, aims) = watcher(edges.lock().expect("fresh mutex").clone());
        let run = edges.clone();
        // The thread never ends, which is what keeps the reporter alive: a
        // dropped reporter is a run that has quit, and the tool has a
        // different true sentence for that.
        std::thread::spawn(move || {
            let mut slots = vec![Some(aiming)];
            loop {
                let asked: Vec<mcp::WireRequest> = reporter.wires().collect();
                if !asked.is_empty() {
                    let mut wires = Vec::new();
                    let mut replies = Vec::new();
                    for mcp::WireRequest { slot, edge, reply } in asked {
                        wires.push((slot, edge));
                        replies.push(reply);
                    }
                    let mut held = run.lock().expect("the run's wiring");
                    let said = rewired(&wires, &mut held, &mut slots, 1);
                    drop(held);
                    for (reply, said) in replies.into_iter().zip(said) {
                        reply.settled(said);
                    }
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        });

        let (failed, said) = call(
            port,
            "wire_input",
            serde_json::json!({"slot": 0, "node": "morph", "input": "far", "to": "drift_shell"}),
        );
        assert!(!failed, "the call came back as a failure: {said}");
        assert!(
            said.contains("wired `morph.far=drift_shell`"),
            "the client was told something other than what the loop did: {said}"
        );
        assert_eq!(
            *edges.lock().expect("the run's wiring"),
            vec![edge("morph", "far", "drift_shell")],
            "the client was answered and the run is not wired with the edge"
        );
        assert_eq!(
            aims.try_recv().expect("the slot was not re-aimed").edges,
            vec![edge("morph", "far", "drift_shell")],
            "the edge was written and nothing was asked to rebuild with it"
        );
    }
}
