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

mod audio;
mod compile;
mod frame;
mod history;
mod mcp;
mod meta;
mod midi;
mod mix;
mod render;
mod scratch;
mod session;
mod setfile;
mod tempo_source;
mod watch;

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use karakuri_engine::binding::{Curve, CONTROL_PREFIX, CURVES, DEFAULT_BPM, NOISE_SIGNAL};
use karakuri_engine::deck::MAX_SLOTS;
use karakuri_engine::swap::Event;
use karakuri_engine::transport::{Sync, Transport};
use karakuri_engine::{
    Binding, Blend, Deck, Gpu, HotSwap, Mask, MaskKind, ParamWrite, Present, Residency, Set,
    Signals, TonemapOp, DEFAULT_BUDGET_MS,
};
use karakuri_midi::Action;
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

/// How wide a wipe's soft edge is.
///
/// Not zero, and not a key. A hard front is an aliased staircase wherever it is
/// not axis-aligned, and this is the narrowest edge that hides that at the
/// resolutions this renders at — narrow enough that a wipe still reads as a
/// wipe rather than a gradient.
const MASK_SOFTNESS: f32 = 0.02;

/// The shape every scheduled fade takes.
///
/// `smooth` rather than `lin`, and the reason is in `binding.rs`: its
/// derivative is zero at both ends, so a fade neither jumps off the floor nor
/// slams into the ceiling. A crossfade of two linear ramps has a visible corner
/// at each end; two smooth ones do not. Not on a key, because the other three
/// curves are for *signals* — a fade wants easing and nothing else, and a
/// fourth cycling key for a choice nobody would revisit is a key in the way.
const FADE_CURVE: Curve = Curve::Smooth;

/// One press of an opacity key. Additive for the same reason as [`GAIN_STEP`],
/// and clamped to `[0, 1]` where gain is not: opacity is a proportion of a
/// blend and there is no such thing as 1.4 of one, while gain is a level into
/// an HDR mix and values above 1.0 are ordinary.
const OPACITY_STEP: f32 = 0.1;

/// One press of an exposure key, as a factor. Multiplicative, because exposure
/// is: a stop is a ratio, and an additive step would be enormous at 0.1 and
/// invisible at 8.0. This is a quarter of a stop, near enough.
const EXPOSURE_STEP: f32 = 1.189_207;

/// Exposure bounds, shared by `--exposure` and the `-`/`=` keys, so a value
/// typed at the command line and a value reached by nudging a key mean the
/// same range. Zero and negative are excluded outright — see [`clamp_exposure`].
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
/// `v` cycles what the output shows — the mix, then each slot alone — and this
/// script does nothing else, because there is nothing else to do: the deck
/// holds one L1 file paired with a sprite renderer in slot 0 and a stroke
/// renderer in slot 1, so cycling the preview *is* the demonstration. What a
/// watcher sees is the same cloud, in the same places, at the same instant,
/// drawn two ways.
///
/// The mix comes first and last on purpose. Both slots composited is the state
/// that shows they are the same geometry — the strokes lie along the dots —
/// and each slot alone is what shows how different the two look when the other
/// is not there to anchor it.
///
/// **Nothing here presses a key that only means something to someone who was
/// told what to expect.** Each `v` produces a visibly different frame on its
/// own, which is the property [`DEMO_SCRIPT`]'s first entry deliberately does
/// not have and has to say so.
const DEMO_LINES_SCRIPT: &[(f32, char)] = &[
    // Five seconds of the mix: strokes and sprites over each other.
    (5.0, 'v'),  // slot 0 alone — sprites *and* strokes, from one simulation
    (10.0, 'v'), // slot 1 alone — strokes, the same elements
    (15.0, 'v'), // back to the mix
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
    /// a watcher handed a one-slot deck sees the script cycle a preview between
    /// the mix and the only thing in it. Overridden the moment any `--set` is
    /// given, so this supplies a scene rather than imposing one.
    fn deck(self) -> Vec<(Named, Vec<Named>)> {
        match self {
            Demo::Transport => Vec::new(),
            // **Two slots, and it has to stay two.** A stack — one slot with
            // both renderers over one simulation — is what several renderers
            // over one geometry now costs, and it is the wrong shape *here*:
            // `DEMO_LINES_SCRIPT` presses the preview key three times to reach
            // the mix, each slot alone, and the mix again, and a preview
            // isolates a **slot**. A one-slot deck has nothing to cycle to, so
            // the script would show the same picture twice and the
            // demonstration would run, look like it worked, and demonstrate
            // nothing — which is the defect the doc above already names.
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

/// Where the store lives when nothing says otherwise. A directory in the
/// working tree rather than under `$HOME`: a session's material belongs beside
/// the session, and a global store shared by every run is a decision an
/// operator should make rather than inherit.
/// Every node of one slot's build: the layer it was sorted onto, its index
/// within that layer, and the hash of the source it was compiled from.
type Nodes = Vec<(&'static str, u32, karakuri_store::hash::Hash)>;

/// Elements per geometry where nothing else says — neither `--capacity` nor the
/// procedure's own `capacity` declaration.
pub const DEFAULT_CAPACITY: u32 = 262144;

const DEFAULT_STORE: &str = ".karakuri";

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
                          cc 5      -> opacity 0     note 32 -> on-air 0
                          note 36 -> prime 0         note 40 -> blend 0
                          note 44 -> preview 0       note 48 -> preview mix
                          note 49 -> tap
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
  --bundle ID           write the Set filed under ID to standard output with
                        every source it names inlined as `src` records, and
                        stop. That file loads on a machine whose store has
                        never held the material, so it is what you send
                        somebody: `--bundle night01 > night01.ndjson`. An
                        artifact this store does not hold refuses the whole
                        bundle naming it, rather than producing a file that
                        looks self-contained and is not
  --unbundle FILE       read a bundled Set file, put its inlined sources in the
                        store, write its Set file, and stop. Every source must
                        hash to the address its `slot` names or the file is
                        refused whole. The id comes from the file, and one
                        already taken is refused rather than overwritten
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
                                      cycling the preview between them. Brings
                                      its own two-slot deck unless --set says
                                      otherwise
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
  v          cycle what the output shows: the mix, then each slot, then the
             mix again. Auditioning — an off-air slot is drawn while it is
             being looked at, so an allocated one shows the still it stopped
             at and a priming one shows what it is warming into. It is never
             stepped by being looked at, so nothing moves that would not have
             moved anyway, and the previewed slot is metered so its level can
             be read before it goes on air. Shown at unity, ignoring its
             faders: what is being judged is the material, not the setting
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

/// The output look: everything the tone mapper is told, in one value, so the
/// window and an offscreen render can be given the same thing and agree.
#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct Look {
    pub op: TonemapOp,
    pub exposure: f32,
    /// Reinhard's only, ignored by the other three. Not on a key: it is one
    /// operator's parameter rather than a control the mix needs.
    pub white_point: f32,
}

impl Look {
    fn name(&self) -> &'static str {
        op_name(self.op)
    }
}

/// Every tone map operator there is. The one list, and its length is in its
/// type, so adding an operator to it is a deliberate act rather than an
/// oversight in a `Vec`.
pub const TONEMAPS: [TonemapOp; 4] = [
    TonemapOp::Clamp,
    TonemapOp::Reinhard,
    TonemapOp::Aces,
    TonemapOp::AgX,
];

/// Both of an operator's spellings: the one a stream and a flag use, and the
/// one a human reads.
///
/// **An exhaustive match, and that is the point.** There were two hand-written
/// lists — `--tonemap`'s parser and `op_name`'s display arm — and the `look`
/// record wanted a third. A lookup over a table would have been one list but
/// would still answer for an operator missing from it, by falling back to
/// something plausible; a match does not compile until every operator has both
/// names. Everything below derives from here, parsing included, so the two
/// directions cannot disagree.
fn spellings(op: TonemapOp) -> (&'static str, &'static str) {
    match op {
        TonemapOp::Clamp => ("clamp", "clamp"),
        TonemapOp::Reinhard => ("reinhard", "Reinhard"),
        TonemapOp::Aces => ("aces", "ACES"),
        TonemapOp::AgX => ("agx", "AgX"),
    }
}

/// How a human reads it. Free to be capitalised the way the papers are,
/// because nothing parses it.
fn op_name(op: TonemapOp) -> &'static str {
    spellings(op).1
}

/// How a stream and a flag spell it. Lower case, stable, and the only spelling
/// anything parses.
pub fn op_wire_name(op: TonemapOp) -> &'static str {
    spellings(op).0
}

/// The wire spelling back to an operator, by searching the one list with the
/// one spelling function. `None` for a name this build does not have, which is
/// the caller's to report against [`op_wire_names`].
pub fn parse_op(name: &str) -> Option<TonemapOp> {
    TONEMAPS
        .iter()
        .copied()
        .find(|op| op_wire_name(*op) == name)
}

/// Every wire spelling, for an error message that says what was available.
pub fn op_wire_names() -> String {
    TONEMAPS
        .iter()
        .map(|op| op_wire_name(*op))
        .collect::<Vec<_>>()
        .join(", ")
}

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

/// A `.kir` named on the command line, with the name the operator gave it.
///
/// **A name belongs to the *use*, not to the procedure** — see `docs/ir-spec.md`,
/// "Naming a source, on the terms HTML gives an `id`". The same lattice twice is
/// one `proc` name and two nodes, so the name is written where the file is
/// spelled and travels with that mention of it.
///
/// `None` is a path written bare. Something still has to address that node, so a
/// name is derived from the procedure once it has compiled, and from the moment
/// it is recorded — derived or written — it *is* the address.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Named {
    name: Option<String>,
    path: PathBuf,
}

impl Named {
    fn bare(path: impl Into<PathBuf>) -> Named {
        Named {
            name: None,
            path: path.into(),
        }
    }

    /// `name=path`, or a bare path.
    ///
    /// **The separator is `=` and not `:`**, which `--param` and `--publish`
    /// already use to mean "a layer and an index follow". `=` is not a path
    /// character anywhere, where `:` is one on Windows.
    fn parse(spelled: &str) -> Result<Named, String> {
        let Some((name, path)) = spelled.split_once('=') else {
            return Ok(Named::bare(spelled));
        };
        check_node_name(name)?;
        if path.is_empty() {
            return Err(format!("`{spelled}` — a name with no file after it"));
        }
        Ok(Named {
            name: Some(name.to_string()),
            path: PathBuf::from(path),
        })
    }
}

/// What a node may be called.
///
/// **Refused rather than mangled**, because a name is an address: something
/// silently renamed is something a mask or a `--param` written against it stops
/// finding, and the author is the only one who can pick the replacement.
fn check_node_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("a node name cannot be empty".to_string());
    }
    // **The layer spellings are reserved**, so that `--param near:radius=1` and
    // `--param L4:0:exposure=1` can never be the same sentence about different
    // things. The two forms are told apart by counting colons, and a node called
    // `L4` would make that count a lie.
    if layer_named(name).is_some() {
        return Err(format!(
            "`{name}` is a layer, so it cannot also be a node's name — a `--param` is told              which of the two it names by the shape of what follows"
        ));
    }
    if let Some(bad) = name
        .chars()
        .find(|c| !c.is_ascii_alphanumeric() && *c != '_' && *c != '-')
    {
        return Err(format!(
            "`{name}` cannot be a node name: `{bad}` is not allowed. Letters, digits, `_` and `-`"
        ));
    }
    Ok(())
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
    /// retired for one.** Bindings belong in a `.set.ndjson`, but nothing
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
    /// `--bundle ID`: write the Set filed under `ID` to standard output with
    /// every source it names inlined, and stop.
    ///
    /// [`ParseOutcome::ListSets`]'s variant for [`ParseOutcome::ListSets`]'s
    /// reason, and the reason is the whole of why it is here: bundling reads a
    /// file and hashes some bytes, and there is no Set to build, no adapter to
    /// request and no window to open. An `Args` field would have to be checked
    /// above every early return in `main` and would be wrong the day somebody
    /// added one more; a variant with no `Args` in it makes reaching a device
    /// not a thing that can be forgotten.
    Bundle {
        store: PathBuf,
        id: String,
    },
    /// `--unbundle FILE`: take a bundled Set file into the store, and stop.
    ///
    /// Here for the same reason, and it compiles: each inlined source goes
    /// through the checker so that a metadata card can be written for it. That
    /// is the check pass, which takes no device — `Set::validate` runs without
    /// one and a single procedure certainly does.
    Unbundle {
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
        // **To standard output, so a shell can redirect it.** A bundle is a
        // thing you *send somebody* — a single self-contained file that works
        // in their store — rather than something the library keeps, so it goes
        // where `> patch.ndjson` puts it and needs no naming rule of its own in
        // `sets/`. The store already holds this Set under an id; a second copy
        // of it there under some derived name would be a second answer to which
        // file is the Set.
        Ok(ParseOutcome::Bundle { store, id }) => match bundled_set(&store, &id) {
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
        Ok(ParseOutcome::Unbundle { store, file }) => match unbundled_file(&store, &file) {
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

fn layer_named(name: &str) -> Option<karakuri_ir::Kind> {
    Some(match name {
        "L1" => karakuri_ir::Kind::L1,
        "L2" => karakuri_ir::Kind::L2,
        "L3" => karakuri_ir::Kind::L3,
        "L4" => karakuri_ir::Kind::L4,
        // **Addressed by its kind, like everything else.** A field has no node,
        // and its params are still an operator's to ride — every procedure that
        // evaluates it writes the same value into its own uniform, so one
        // address reaches all of them.
        "Field" => karakuri_ir::Kind::Field,
        _ => return None,
    })
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
        capacity: 262_144,
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
        store: PathBuf::from(DEFAULT_STORE),
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
    let mut bundle: Option<String> = None;
    let mut unbundle: Option<PathBuf> = None;
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
                // `--set` already fail loudly on a bad value; this now does
                // the same, and the bound matches what `-`/`=` clamp to at
                // runtime so a value typed at the command line and one
                // reached by nudging a key mean the same range.
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
            "--bundle" => bundle = Some(value_for("--bundle", &mut it)?),
            "--unbundle" => unbundle = Some(PathBuf::from(value_for("--unbundle", &mut it)?)),
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
    if let Some(id) = bundle {
        return Ok(ParseOutcome::Bundle {
            store: args_out.store,
            id,
        });
    }
    if let Some(file) = unbundle {
        return Ok(ParseOutcome::Unbundle {
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
            "--load-set names the material and so do the `.kir` paths beside it; give one              or the other"
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

/// One deck slot's material, sorted into the chain a `Set` is built from: the
/// procedure that simulates, the deformations between, and the renderers drawn
/// over the result.
///
/// **The sorting happens here rather than on the command line**, because every
/// `.kir` declares its own `kind` — so `--set` is one comma-separated list and
/// the loader reads which is which off the files. Order *within* a kind is list
/// order, which is chain order for L2s and draw order for L4s.
struct Material {
    /// **The geometry sources**, in the order their paths appeared. At least
    /// one; several is a merge.
    l1s: Vec<karakuri_ir::typed::Checked>,
    l2s: Vec<karakuri_ir::typed::Checked>,
    /// **The cameras, in the order their paths appeared.** Empty leaves the
    /// slot looking from the built-in orbit, which is a node all the same — so
    /// a Set has at least one camera however this list comes out.
    l3s: Vec<karakuri_ir::typed::Checked>,
    /// **The fields, in the order their paths appeared.** Empty for a slot that
    /// evaluates none.
    fields: Vec<karakuri_ir::typed::Checked>,
    l4s: Vec<karakuri_ir::typed::Checked>,
    /// What each of those is called, in the same per-layer shape.
    names: Names,
}

/// A name for every node of a slot, in the shape the procedures themselves are
/// passed in.
///
/// **Per layer rather than one list in node order**, because the node order is
/// the engine's — `slot_of` and `nodes_of` decide it — and a caller that
/// reproduced it here would be a second place for a fact this project has
/// already been bitten by twice.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Names {
    pub l1s: Vec<Option<String>>,
    pub l2s: Vec<Option<String>>,
    /// **A list, like the renderers'.** A slot holds as many cameras as its
    /// files declare, and the built-in orbit is a node beside them — one
    /// nobody can name from the command line, since a name is written beside
    /// a path and the built-in has none. It is called `orbit`; see
    /// `karakuri_engine::set::BUILTIN_CAMERA`.
    pub l3s: Vec<Option<String>>,
    pub l4s: Vec<Option<String>>,
    pub fields: Vec<Option<String>>,
}

impl Names {
    /// Every name that was actually written, which is the only thing worth
    /// checking for a collision: a derived one is disambiguated where it is
    /// derived, in `Set::build_many`.
    fn written(&self) -> impl Iterator<Item = &String> {
        self.l1s
            .iter()
            .flatten()
            .chain(self.l2s.iter().flatten())
            .chain(self.l3s.iter().flatten())
            .chain(self.l4s.iter().flatten())
            .chain(self.fields.iter().flatten())
    }

    /// **Unique within a slot**, which is the scope a name resolves in: a Set is
    /// what holds the nodes, so two slots may each have a `near` and neither is
    /// ambiguous.
    ///
    /// Refused rather than disambiguated. A derived name is disambiguated
    /// where it is derived — that is what `-2` is for — so a collision reaching
    /// here is two *written* names, and picking one for the author would leave
    /// a `--param` pointing at whichever the tie-break preferred.
    fn check_unique(&self) -> Result<(), String> {
        let mut seen: Vec<&str> = Vec::new();
        for name in self.written() {
            if seen.contains(&name.as_str()) {
                return Err(format!(
                    "two nodes are both called `{name}` — a name addresses one node in a slot"
                ));
            }
            seen.push(name);
        }
        Ok(())
    }
}

/// Where one of a slot's files ended up: the layer its own `kind` declaration
/// puts it on, and which node of that layer it is, beside the name and path it
/// was spelled with.
///
/// **Kept beside the compiled [`Material`] rather than worked out again.** A
/// Set file records a node's layer and its index, and a run that answered "what
/// layer is this file on" once for the engine and once for the file it saves
/// would hold two answers to one question — the shape this project has been
/// bitten by twice. See [`setfile::Node`], which is this as the record.
// **`Eq` and not merely `PartialEq` is what the metadata card costs**: a record
// carries the declared range as `f32`, so the lines are comparable and not
// totally so. Nothing asks for `Eq` — no `Placed` is a map key — and the
// comparison that is used, in the tests below, is unchanged: two nodes with the
// same source have the same card, because the card is a function of it.
#[derive(Clone, Debug, PartialEq)]
struct Placed {
    named: Named,
    /// The name the procedure itself declares, which is not the name in
    /// `named`: that one belongs to the *use* and is `None` for a bare path.
    ///
    /// **Read where the file was compiled, not scanned for again.** The edit
    /// history files a version under the procedure's name and a session's
    /// `procedure` record carries it, and both of them are looking at a node
    /// this already knows the layer and index of — asking a second reader would
    /// be a second answer to a question already settled here.
    proc: String,
    layer: karakuri_ir::Kind,
    index: u32,
    /// **The text this node was compiled from**, carried from the read that
    /// produced the `Checked` beside it.
    ///
    /// This is the *one* derivation of "what bytes is this node running", and
    /// everything downstream is a function of it: [`Placed::hash`] is the
    /// address a live save writes and a `procedure` record names,
    /// [`Placed::put`] is how those bytes reach the store, and the edit history
    /// files this same buffer. It used to be re-read from `named.path` by each
    /// of them, which made the answer whatever the disk happened to hold at the
    /// moment they asked — and between the compile and the first frame sit the
    /// adapter request, the deck build, `measure_slots`, and the audio, MIDI,
    /// tempo and **MCP server** starts. Anything rewriting a `.kir` in that
    /// window moved the launch hash onto bytes the deck had never compiled; if
    /// the rewrite did not compile, no watcher ever corrected it, and `k` wrote
    /// a Set naming a procedure that had never reached the screen and did not
    /// load back.
    ///
    /// **Shared rather than copied**, because a slot's `Placed` list is cloned
    /// into [`Live::startup`] and crosses onto a save thread; the bytes
    /// themselves are read once and never again.
    source: std::sync::Arc<str>,
    /// **This node's metadata card**, built from the `Checked` the compile
    /// produced — see [`crate::meta::card`].
    ///
    /// It rides here for the reason `source` does, and it is the same reason
    /// twice: this is the one compile the run will do of these bytes, and the
    /// card is a function of it. The alternative was to build it where the
    /// artifact is written, which would mean **re-compiling the source at save
    /// time** — a second pass whose answer can differ from the one on screen
    /// the moment anything about the checker is version-dependent, and a
    /// compile on the path of a keypress besides. `Checked` itself is not
    /// carried: what a card says is decided once, and holding the whole checked
    /// tree per node to re-derive it would be holding the question instead of
    /// the answer.
    ///
    /// Shared, because a slot's list is cloned onto the save thread.
    meta: std::sync::Arc<[karakuri_store::ndjson::Line]>,
}

impl Placed {
    /// **Where this node's source is addressed**, derived from the bytes above
    /// and from nothing else.
    ///
    /// A method rather than a field beside `source`, because a hash is a pure
    /// function of the bytes: computed here it cannot disagree with them, where
    /// a stored copy would be a second thing to keep true. It is a few
    /// kilobytes hashed at launch and again per save, which is not a rate
    /// anything here is bounded by.
    fn hash(&self) -> karakuri_store::hash::Hash {
        karakuri_store::hash::Hash::of(self.source.as_bytes())
    }

    /// This node as [`setfile::Node`]. **No store and no disk** — the address
    /// comes off the bytes the compile read.
    fn node(&self) -> setfile::Node {
        setfile::Node {
            hash: self.hash(),
            layer: self.layer,
            index: self.index,
            name: self.named.name.clone(),
        }
    }

    /// **Put this node's source in `store`**, so that a file or a record naming
    /// [`Placed::hash`] resolves on the way back in.
    ///
    /// **Separate from [`Placed::node`], and called later than it.** Knowing
    /// what a slot is running costs nothing and every windowed run needs it;
    /// writing the bytes down creates a directory and a file, and only two
    /// callers need that — a save that actually happened, and a run recording a
    /// session, whose `procedure` records a replay has to resolve. Folding the
    /// two together is what made a plain windowed run create a store it was
    /// never asked for; see [`Running::at_launch`].
    ///
    /// **The card goes down beside the artifact, and a card that will not write
    /// does not fail the put.** The artifact is the thing; its metadata is
    /// derived from the `.kir` plus a compile pass and regenerates on the next
    /// one, so a store holding the source and no card holds everything that
    /// cannot be recovered. Failing here instead would mean an operator losing
    /// a save — or a session losing a `procedure` record's source — over a file
    /// nothing has read yet. It is still said out loud: silence would leave a
    /// library quietly thinning out as it grew.
    fn put(
        &self,
        store: &karakuri_store::store::Store,
    ) -> Result<karakuri_store::hash::Hash, String> {
        let hash = store
            .put_artifact(self.source.as_bytes())
            .map_err(|e| format!("{}: {e}", self.named.path.display()))?;
        put_meta(store, &hash, &self.meta);
        Ok(hash)
    }
}

/// **Write an artifact's metadata card, and never let it stop a save.**
///
/// The one place either put path says this, so that the judgement — the card is
/// derived and the artifact is not — is made once and the sentence is one
/// sentence. See [`Placed::put`], which is the other caller's other half.
///
/// The hash is passed rather than recomputed: the caller has just put the bytes
/// under it, and deriving it again here would be a second answer to which
/// artifact this card is for.
///
/// **It hands back what it said, and both callers drop it.** The policy — a
/// card that will not write is reported and does not fail the save — was
/// asserted in prose and nowhere else, because a sentence that is only printed
/// is a sentence no test can hold. Returning it costs one `Option` and buys
/// `live_save_tests::a_card_that_will_not_write_is_said_and_does_not_fail_the_save`,
/// which reads it back. `None` is a card on disk.
fn put_meta(
    store: &karakuri_store::store::Store,
    hash: &karakuri_store::hash::Hash,
    card: &[karakuri_store::ndjson::Line],
) -> Option<String> {
    let e = store.write_meta(hash, card).err()?;
    let said = format!(
        "  the metadata for {}: {e} — the artifact is stored and the library will \
         regenerate its card from the source on the next compile",
        hash.short(12)
    );
    eprintln!("{said}");
    Some(said)
}

/// **Sort one slot's compiled procedures by the `kind` each declares**, keeping
/// list order within a kind.
///
/// **The one place that answers "which layer is this file on, and which node of
/// it".** Both ways into a slot come through here: [`sort_slot`] compiles from
/// paths at startup, and [`crate::watch::Watch::poll`] compiles from text it has
/// already read — it needs the bytes for the edit history and for the artifacts
/// a session stores. Loading is the only thing they do differently, so the seam
/// is after the compile and this takes procedures rather than paths. The two
/// used to hold a copy each of this match, and they had already drifted: the
/// rebuild still took its head for the L1 whatever the file declared.
///
/// **The first procedure is the first source**, and every later `kind L1` is
/// another one. Each simulates independently — its own `seed` from zero, its own
/// hash salt, its own compaction — and the renderers draw all of them. See
/// `docs/ir-spec.md`, "Multiple L1 sources".
///
/// The head is sorted by its declaration like everything after it. It used to
/// be taken as the L1 whatever it said, which cost nothing while the engine was
/// the only reader — it refuses a non-L1 there with `WrongKind` — and starts
/// costing as soon as a Set file records the layer: an L2 written down as
/// `slot L1` is a file that reads back as a Set nobody assembled.
///
/// **`Err` is a sentence and not an exit**, because the two callers answer a
/// refusal differently and that difference is the only reason there were ever
/// two of these: a startup that cannot assemble its slot has nothing to run and
/// stops, while a rebuild that cannot leaves the Set that *is* running alone.
/// Each prefixes the slot it is about and decides.
fn sort_compiled(
    compiled: Vec<(Named, karakuri_ir::typed::Checked, std::sync::Arc<str>)>,
) -> Result<(Material, Vec<Placed>), String> {
    // Taken before the loop consumes the list. The "nothing draws" refusal
    // names the file the slot was given, which is the one an operator looks at
    // first — and by then it has been moved from.
    let head = compiled
        .first()
        .map(|(named, ..)| named.path.display().to_string());
    // **A name per node, in the same per-layer shape the engine takes its
    // procedures in.** Not one flat list in node order: that order is the
    // engine's, and a caller that reproduced it would be the second place a
    // fact this project has already been bitten by lives.
    let mut names = Names::default();
    let mut l1s = Vec::new();
    let mut l2s = Vec::new();
    let mut l3s: Vec<karakuri_ir::typed::Checked> = Vec::new();
    let mut fields: Vec<karakuri_ir::typed::Checked> = Vec::new();
    let mut l4s = Vec::new();
    let mut placed = Vec::new();
    for (named, checked, source) in compiled {
        let name = named.name.clone();
        let proc = checked.name.clone();
        // **Each arm hands back the `Checked` it just filed**, because the
        // card below is read off it and the arm is where it otherwise goes out
        // of reach. This loop is the last point in the run holding both the
        // checked procedure and the bytes it came from — see [`Placed::meta`]
        // for why the card is kept and the tree is not.
        let (layer, index, filed) = match checked.kind {
            karakuri_ir::Kind::L2 => {
                names.l2s.push(name);
                l2s.push(checked);
                (karakuri_ir::Kind::L2, l2s.len() - 1, l2s.last())
            }
            karakuri_ir::Kind::L4 => {
                names.l4s.push(name);
                l4s.push(checked);
                (karakuri_ir::Kind::L4, l4s.len() - 1, l4s.last())
            }
            // **A second camera is a second camera**, and this was the last
            // refusal in this file that said otherwise. It said a slot looks
            // from one viewpoint, which was true of the plumbing and not of
            // the material: a renderer declares `uses view : Camera` and an
            // `edge` names which node fills it, so two cameras are two nodes
            // with two names and nothing has to arbitrate between them. A
            // renderer that names none draws from the first, which is what it
            // has always drawn from.
            karakuri_ir::Kind::L3 => {
                names.l3s.push(name);
                l3s.push(checked);
                (karakuri_ir::Kind::L3, l3s.len() - 1, l3s.last())
            }
            // **A second field is a second field**, not a mistake — the last
            // refusal in this file that said otherwise, and it said so about
            // the plumbing rather than about the material. A caller declares
            // `uses shape : Field` and an `edge` names which node fills it, so
            // two fields are two nodes with two names and nothing has to
            // arbitrate between them. What it took was a `Vec` here, in
            // `Loaded`, in `Wiring` and in the engine — an `Option` apiece was
            // all that was left of "there is exactly one, so it needs no name".
            karakuri_ir::Kind::Field => {
                names.fields.push(name);
                fields.push(checked);
                (karakuri_ir::Kind::Field, fields.len() - 1, fields.last())
            }
            // **A second L1 is a second source**, not a mistake. Each one
            // simulates independently — its own `seed` from zero, its own hash
            // salt, its own compaction — and the renderers draw all of them,
            // and rebuilding one needs nothing a first does not: a request
            // carries a list, and every record of a build addresses a node as
            // `(slot, layer, index)`. The rebuild's copy of this used to refuse
            // — a slot with two geometries started, then printed a refusal on
            // every save for the rest of the run with the picture frozen at its
            // startup build.
            karakuri_ir::Kind::L1 => {
                names.l1s.push(name);
                l1s.push(checked);
                (karakuri_ir::Kind::L1, l1s.len() - 1, l1s.last())
            }
        };
        let checked = filed.expect("the procedure was pushed onto that layer's list above");
        // **The node first, and its card read off the node.** A card names the
        // artifact it describes by its hash, and [`Placed::hash`] is where a
        // node's address is derived — spelling `Hash::of(source)` here as well
        // would be a second derivation of one fact, which is the defect this
        // file has been bitten by twice and the reason `hash` is a method
        // rather than a field beside `source`. `meta` is therefore empty for
        // exactly one statement, and nothing can observe a `Placed` in that
        // state: the node is not reachable until it is pushed.
        let mut node = Placed {
            named,
            proc,
            layer,
            index: index as u32,
            source,
            meta: Vec::new().into(),
        };
        node.meta = meta::card(&node.hash(), checked).into();
        placed.push(node);
    }
    if l4s.is_empty() {
        return Err(match head {
            Some(head) => format!(
                "nothing here draws — {head} names no L4, and a Set with no \
                 renderer has no frame to give"
            ),
            // Nothing at all to sort is the same refusal with no file to point
            // at. Neither caller can reach it — a slot is spelled with a head —
            // and answering it here is cheaper than making that a precondition.
            None => "nothing here draws — a Set with no renderer has no frame to give".to_string(),
        });
    }
    names.check_unique()?;
    Ok((
        Material {
            l1s,
            l2s,
            l3s,
            fields,
            l4s,
            names,
        },
        placed,
    ))
}

/// **Compile one slot's files and sort them**, which is [`sort_compiled`] with
/// the loading in front of it and the exit behind it.
///
/// Fatal on anything the sort refuses, and fatal here rather than at the build,
/// because the file is what an operator can fix. A run that cannot assemble a
/// slot has no picture to keep showing — which is exactly what the rebuild
/// path, over the same sort, does instead.
fn sort_slot(slot: usize, l1: &Named, rest: &[Named]) -> (Material, Vec<Placed>) {
    let named: Vec<String> = rest.iter().map(|p| p.path.display().to_string()).collect();
    eprintln!(
        "  slot {slot}: {} + {}",
        l1.path.display(),
        named.join(" + ")
    );
    // **The text each file was compiled from travels with it**, because it is
    // the only copy this run will ever make of those bytes — see [`Placed`],
    // which is where it lands and why re-reading the path later was wrong.
    let compiled: Vec<(Named, karakuri_ir::typed::Checked, std::sync::Arc<str>)> =
        std::iter::once(l1)
            .chain(rest)
            .map(|named| match compile::load(&named.path) {
                Ok((checked, src)) => (named.clone(), checked, std::sync::Arc::from(src.as_str())),
                Err(report) => {
                    eprintln!("{report}");
                    std::process::exit(1);
                }
            })
            .collect();
    match sort_compiled(compiled) {
        Ok(sorted) => sorted,
        Err(e) => {
            eprintln!("slot {slot}: {e}");
            std::process::exit(1);
        }
    }
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
        args.look.name(),
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
        Ok(Some(mix::Change::Preview { slot })) => deck.set_preview(slot),
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
            offset_beats,
        })) => {
            if let Err(refusal) = deck.set_transport(slot, sync, anchor_bpm, offset_beats) {
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
/// [`crate::setfile::summarise`]: one derivation, two renderings. What a node
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
/// **A bundle goes to standard output**, which is what this returning a
/// `String` is: it is a file you *send somebody* — one self-contained patch
/// that loads in a store which has never held the material — so it belongs
/// where `karakuri-cli --bundle night01 > night01.bundle.ndjson` puts it. A
/// store directory would need a naming rule of its own for it, and a second
/// copy of a Set sitting beside the Set is a second answer to which of them is
/// the file.
///
/// **Opening nothing that is not there**, on [`listed_sets_at`]'s terms: a flag
/// that only reads must not leave a store behind to report that a Set is
/// missing from it. Here it is an error rather than an answer, because a bundle
/// of a Set that does not exist is not a bundle.
fn bundled_set(root: &std::path::Path, id: &str) -> Result<String, String> {
    if !root.exists() {
        return Err(format!(
            "no store at `{}`, so there is no set `{id}` to bundle — check `--store`",
            root.display()
        ));
    }
    let store = karakuri_store::store::Store::open(root)
        .map_err(|e| format!("store `{}`: {e}", root.display()))?;
    Ok(setfile::bundle(&store, id)?
        .iter()
        .map(|line| format!("{}\n", line.as_str()))
        .collect())
}

/// **A bundle somebody sent you, into this store**, and the report of what
/// happened.
///
/// The store *is* opened where it is not there, unlike [`bundled_set`]: this is
/// a write, and establishing the layout under a root an operator named is what
/// every other writing path here does.
fn unbundled_file(root: &std::path::Path, file: &std::path::Path) -> Result<String, String> {
    let lines = karakuri_store::ndjson::read(file)
        .map_err(|e| format!("reading `{}`: {e}", file.display()))?;
    let store = karakuri_store::store::Store::open(root)
        .map_err(|e| format!("store `{}`: {e}", root.display()))?;
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
            "  only slot 0 is saved: a Set file describes one Set, and which Sets a deck              is holding belongs to a session"
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
    // be a function of its arguments. See `scratch.rs` for the rest of the
    // reasoning, including what happened when there was no such place.
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
            let written: Result<Vec<PathBuf>, String> = loaded
                .nodes()
                .map(|(checked, src)| scratch::place(&root, &checked.name, src))
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
        match scratch::materialise(
            &root,
            args.sets
                .iter_mut()
                .flat_map(|(l1, l4s)| std::iter::once(l1).chain(l4s.iter_mut()))
                .map(|n| &mut n.path),
        ) {
            Ok(dir) => eprintln!(
                "scratch: {} — the deck runs from copies here, so the files you named are \
                 not written to. Point an editor at these",
                dir.display()
            ),
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
            let mut deck = build_deck(&gpu, &procs, &args, false, false, w, h, None, None);
            eprintln!(
                "rendering the mix of {} Set{}, {w}x{h}, {} frames, \
                 {} at exposure {:.2} -> {}",
                deck.slot_count(),
                if deck.slot_count() == 1 { "" } else { "s" },
                args.frames,
                args.look.name(),
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
) -> Deck {
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
    let swaps: Vec<_> = procs
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
                // One worker and one watcher per slot, over that slot's own
                // two files. That is what makes "the slot whose files changed"
                // the thing that rebuilds: no slot can see another's edit.
                HotSwap::new(&gpu.device, &gpu.queue, set, args.budget_ms, {
                    let watcher = watch::Watch::new(
                        slot,
                        // **With the names, not only the paths.** A rebuild
                        // resolves its edges against them, and a watcher that
                        // handed the sort bare paths would rename every node
                        // on the first save.
                        args.sets[slot].0.clone(),
                        args.sets[slot].1.clone(),
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
                        args.capacity_given.then_some(args.capacity),
                        salts[slot]
                            .first()
                            .copied()
                            .unwrap_or_else(|| seed_for(slot)),
                        // **Restated on every rebuild rather than derived
                        // there.** A slot filled from a Set file is running at
                        // the salts that file recorded, and a rebuild that
                        // derived its own would change every colour in it on the
                        // next save of a `.kir`.
                        salts[slot].clone(),
                        // **The same reading the Set was built with**, and
                        // stated as an `Orbit` rather than as the `Option` it
                        // was read as: a slot that loaded no `camera` record is
                        // running at `Orbit::default()`, because that is what
                        // `Set::build_many` builds and what leaving it
                        // unassigned above therefore means. See `Watch::camera`
                        // for why the option buys nothing past this line.
                        camera.unwrap_or_default(),
                        args.overrides.clone(),
                        args.published.clone(),
                        args.bindings.clone(),
                        args.edges.clone(),
                    );
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
                })
            } else {
                HotSwap::fixed(set)
            }
        })
        .collect();
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
    deck
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
                if set.write_param(write) == 0 {
                    eprintln!("  no parameter named `{}`, ignoring", write.key);
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
    // cannot yet. See the note in `session.rs`.
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

    /// How many steps to advance by, and **only called by a frame that is going
    /// ahead.** `last` moves here and nowhere else, so an abandoned frame
    /// leaves the interval it did not use to be counted by the next one — which
    /// is why the acquire has to come first.
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

/// One node of a live save: its layer as a record spells it, which node of that
/// layer, the address its source has, the name the operator gave the file, and
/// — for a node still at the version the run launched with — the bytes to put
/// in the store on the way past.
///
/// **The name is the part no hash could carry**, which is why this is not
/// [`Nodes`]: a name belongs to the *use* rather than to the procedure, so it
/// comes from the command line and travels beside the address rather than
/// inside it.
struct SavedNode {
    layer: &'static str,
    index: u32,
    hash: karakuri_store::hash::Hash,
    name: Option<String>,
    /// The launch source, when this node is still running it, and `None` when a
    /// build put the version there instead.
    ///
    /// **Which is the whole of what "lazily" means.** The watcher puts what it
    /// builds in the store as it builds it, so a rebuilt node's bytes are
    /// already there and there is nothing to carry. A node still on its launch
    /// version has bytes that live only in this process — see [`Placed`] — so
    /// they ride along and reach the store at the moment a file names them.
    /// That is why a windowed run creates nothing until somebody presses `k`.
    ///
    /// These are the *compiled* bytes and never a re-read of the path, so a
    /// `.kir` rewritten since launch cannot reach a saved file.
    source: Option<std::sync::Arc<str>>,
    /// The card that goes down with those bytes, carried on exactly the same
    /// condition and for the same reason — see [`Placed::meta`]. `None`
    /// wherever `source` is `None`: a rebuilt node's card was written by the
    /// build that stored it, and there is nothing here to add.
    ///
    /// **That is held by construction rather than asserted.** [`live_sources`]
    /// asks the one predicate once and takes the pair off it, because it used
    /// to ask it twice — two derivations of "are these the bytes on screen"
    /// with nothing keeping them equal, where [`Sources::into_nodes`] writes the
    /// card *inside* the branch that puts the source and would have dropped a
    /// card whose `source` had gone `None` without a word.
    ///
    /// A field beside `source` rather than derived from it, because deriving it
    /// would mean re-compiling the source on the save thread — see
    /// [`Placed::meta`], which declined the same thing on the same grounds.
    meta: Option<std::sync::Arc<[karakuri_store::ndjson::Line]>>,
}

/// **Where one live save's sources come from**: the hashes of the versions this
/// slot is running, with the name the operator gave each file beside them.
///
/// **One answer, not two.** This used to be an enum — the landed hashes where a
/// build had landed, and the startup *paths* where none had — and the second arm
/// was a save that read the disk. Reading the disk answers a different question:
/// a slot rolled back to what it launched with, an edit that never compiled, and
/// a run with no watcher at all are all states where the file and the picture
/// disagree, and every one of them wrote down a version nobody had seen. The
/// hashes are seeded at launch instead — see [`Running::at_launch`] — so there
/// is one representation of "what bytes is this node running", derived once
/// from the text the compile read and a hash from the first frame onward.
///
/// See `Live::save_set`. Owned, because it crosses onto the thread that does the
/// store I/O.
struct Sources(Vec<SavedNode>);

impl Sources {
    /// How many nodes this names. Zero is a slot with nothing behind it — see
    /// `Live::save_set`, which refuses rather than writing a file describing no
    /// Set.
    fn len(&self) -> usize {
        self.0.len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The nodes a Set file will name, **with every one of them in the store**.
    ///
    /// **Nothing here reads a `.kir`**, which is what collapsing the two arms
    /// bought: a node a build put there was stored by the watcher that built
    /// it, and a node still on its launch version carries the bytes the compile
    /// read. Either way the address was derived from bytes this process has
    /// held all along, and the only thing left to do is make sure the store has
    /// them — `put_artifact` is content-addressed, so putting one that is
    /// already there costs an `exists` and writes nothing.
    ///
    /// **This is the moment a windowed run first touches the store.** Seeding
    /// it at launch instead created a directory for every run whether or not
    /// anything was ever saved; see [`Running::at_launch`].
    fn into_nodes(
        self,
        store: &karakuri_store::store::Store,
    ) -> Result<Vec<setfile::Node>, String> {
        self.0
            .into_iter()
            .map(|node| {
                let SavedNode {
                    layer,
                    index,
                    hash,
                    name,
                    source,
                    meta,
                } = node;
                let layer = layer_named(layer)
                    .ok_or_else(|| format!("a node on layer `{layer}` cannot be saved"))?;
                if let Some(source) = source {
                    store
                        .put_artifact(source.as_bytes())
                        .map_err(|e| format!("the source of a `{layer:?}` node: {e}"))?;
                    // **Beside the bytes, on [`Placed::put`]'s terms**: the
                    // card is derived and the artifact is not, so a card that
                    // will not write is said and not raised. Inside the `if`
                    // because it is the same condition — a node whose bytes
                    // were already in the store has a card there too.
                    if let Some(meta) = meta {
                        put_meta(store, &hash, &meta);
                    }
                }
                Ok(setfile::Node {
                    hash,
                    layer,
                    index,
                    name,
                })
            })
            .collect()
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

/// **How long the end of a run waits for saves still being written.**
///
/// Long enough that a save of a few dozen lines and a handful of artifacts
/// finishes on any disk that is answering, and short enough that one which is
/// not answering costs a quit five seconds rather than the window. See
/// [`Live::awaited_saves`] for why the wait exists at all.
const SAVE_WAIT: Duration = Duration::from_secs(5);

/// **One live save, from the frame that asked for it to the file on disk.**
struct Save {
    slot: usize,
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
        setfile::save(&store, &id, values.saving())
    }
}

/// What a live save came back with, at the frame it arrives.
struct Saved {
    slot: usize,
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

/// **No such slot**, in the words every surface says it in.
///
/// Extracted where a second caller appeared, rather than copied to it: focusing
/// a slot that does not exist and saving one are the same mistake and were about
/// to be two sentences about it.
///
/// **And "every surface" is now literally every one of them**, which it was not
/// when this sentence was first written. There were four spellings of one
/// refusal — the keys said `no slot 9: this deck holds slots 0-3`, `--mcp` said
/// `holds 0-3`, MIDI said `no slot 9 — this deck holds slots 0-3`, and a `gain`
/// record naming a slot said `slot 9: this deck holds slots 0-3` — so a model
/// calling `save_set {"slot":9}` and an operator pressing `9` got different
/// sentences for the same mistake on the same control. That was tolerable while
/// each surface reached different controls; it stopped being tolerable when
/// `save_set` made one control reachable from two of them: the refusals are
/// the same sentences whoever meets them — see
/// `docs/principles/0061-a-refusal-a-person-can-reach-from-two-surfaces-is-one-sentence.md`.
/// `mcp.rs`, `midi.rs` and
/// `mix.rs` all call this now. Each of them pins it with an `assert_eq!`
/// against this function rather than trusting this comment — see
/// `mcp::tests::a_slot_a_layer_and_a_renderer_resolve_and_anything_else_is_refused`,
/// `mcp::wire_tests::a_save_for_a_slot_that_does_not_exist_is_refused_here`,
/// `midi::tests::an_unmapped_control_and_a_missing_slot_are_each_reported_once`
/// and `mix::tests::a_slot_past_the_deck_is_refused_with_the_range_it_missed`.
/// Every one of them asked only `contains(...)` before, which is why four
/// spellings could live side by side unnoticed.
///
/// The engine's own `no slot` messages are deliberately *not* routed here: they
/// are `assert!`s on a call that should never have been made, addressed to
/// whoever is holding the debugger, and a refusal an operator reads and a panic
/// a programmer reads are two audiences that happen to share a phrase.
fn no_such_slot(slot: usize, slot_count: usize) -> String {
    match slot_count {
        // Cannot happen — a run with no slots does not reach a window — and
        // written anyway, because `slot_count - 1` on it is an underflow and a
        // panic, which is what the arm that "cannot happen" costs when the shape
        // around it changes.
        0 => format!("no slot {slot}: this deck holds none"),
        n => format!("no slot {slot}: this deck holds slots 0-{}", n - 1),
    }
}

/// **A renderer the slot does not draw with**, in the words every surface says
/// it in.
///
/// [`no_such_slot`]'s shape, one address down: the thing named, then what there
/// was to name. A selection is the first control that addresses *inside* a
/// slot, so it is the first refusal that needed this — and it is a free
/// function beside that one rather than a sentence in `Live`, because the key
/// press and a replayed record both meet it and telling one operator two
/// stories about one mistake is what `no_such_slot` exists to have stopped.
///
/// **The count comes from the Set on screen, not from the flags the run
/// started with**, which is what makes it true after a hot swap: a rebuilt
/// slot draws with however many renderers its new sources declare.
fn no_such_renderer(slot: usize, at: usize, count: usize) -> String {
    match count {
        // Cannot happen — a Set with no renderer does not build — and written
        // anyway, because `count - 1` on it underflows and panics, which is
        // what the arm that "cannot happen" costs when the shape around it
        // changes. The same reasoning as [`no_such_slot`]'s empty deck.
        0 => format!("no renderer {at}: slot {slot} draws with none"),
        n => format!(
            "no renderer {at}: slot {slot} draws with renderers 0-{}",
            n - 1
        ),
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

/// **Why a slot has nothing to save**, in the words the operator is given.
///
/// **Named, and with the flag that changes the answer.** Every refusal around
/// this one names the file or the range it is about; this one used to name
/// neither the id the run came from nor anything the operator could do, which
/// leaves them pressing a key that reports a fact about the world rather than a
/// way out of it.
///
/// A free function over the two facts it turns on, for the reason
/// [`drained_saves`] is one: it has to reach a model as well as a terminal now,
/// which makes it worth a test, and `Live` needs a window and a GPU.
///
/// `no_files` is whether this slot has any startup sources at all — see
/// `Live::startup`, which is empty exactly for a slot filled straight from a Set
/// file by hash.
fn nothing_to_save(slot: usize, loaded_set: Option<&str>, no_files: bool) -> String {
    match loaded_set {
        // **Both flags, because `editable()` is both.** It is `editable()` that
        // materialises a loaded Set into the scratch and puts it in
        // `args.sets`, and that is `--watch || --mcp` — so `--load-set X --mcp
        // PORT` with no `--watch` already saves like any other slot. Naming only
        // `--watch` sent an operator who had `--mcp` off to restart a set for a
        // flag they did not need.
        Some(id) if no_files => format!(
            "slot {slot}: nothing to save — it was filled from set `{id}` by hash, with no \
             files behind it and nothing able to rebuild it. Start the run with `--watch` \
             or `--mcp` and this slot saves like any other"
        ),
        _ => format!(
            "slot {slot}: nothing to save — this slot's sources are not in the store, which \
             was said at startup, and no rebuild of it has landed since"
        ),
    }
}

/// **A save has been taken and named**, said to the terminal and to whoever
/// asked for it if that was not a hand. Returns the id it will be filed under.
///
/// Said before the store thread starts, because the operator pressed a key and
/// the answer to "did it take" is owed now rather than when the disk gets round
/// to it. Where it went is said on arrival — see [`Live::took_save`].
///
/// **The same sentence to a waiting client, and this half is the one a timeout
/// depends on.** [`mcp::Reply`] carries two messages because "accepted, under
/// this id, outcome not yet known" is a third fact the protocol's one boolean
/// cannot hold: a client whose deadline passes with this message in hand is
/// told to go looking under the id, and one without it is told *nothing was
/// saved and asking again is safe* — which is a false claim to a model about a
/// save that is running and will land.
///
/// **A free function over the four facts it turns on**, for the reason
/// [`nothing_to_save`] and [`drained_saves`] are, and the reason bites harder
/// here. Those are refusals — said *instead of* a save, and reachable without a
/// window. This is said *during* one, and inline it sat below
/// [`playing_values`], which reads `deck.slot(slot)` and is the single line of
/// [`Live::save_set`] that genuinely needs a GPU. So it was the one half of the
/// accept-then-settle sequence no test could reach: deleting the `accepted`
/// call left the whole suite green, because every `--mcp` test drives a
/// stand-in loop that sends `accepted` itself. Above that line it is testable,
/// and `mcp::tests::a_save_the_loop_has_taken_names_its_id_to_a_client_that_times_out`
/// is what deleting the call now costs.
fn accepted_save(
    slot: usize,
    id: Option<String>,
    sources: &Sources,
    root: &std::path::Path,
    reply: Option<&mcp::Reply>,
) -> String {
    // **A name is a stamp, because a key press cannot type one.** See
    // `history::stamped_id`, whose convention this is: an operator looks for
    // the time they saved it. A client that named one gets the name it named —
    // see `mcp::checked_id` on why a set filed under a name its caller did not
    // ask for is the worse answer.
    let id = id.unwrap_or_else(history::stamped_id);
    let said = format!(
        "slot {slot}: saving {} node{} as set `{id}` in {}",
        sources.len(),
        if sources.len() == 1 { "" } else { "s" },
        root.display()
    );
    eprintln!("{said}");
    if let Some(reply) = reply {
        reply.accepted(&said);
    }
    id
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
    /// The control surface, when `--midi-in` asked for one. Every action it
    /// produces ends in the same method a key press ends in — see
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
    /// nothing to read back; and unlike a param or a salt it cannot move during
    /// a run — no key and no MCP tool rewires a `uses` slot, and every rebuild
    /// restates this same list. So this is a copy of a value, not a second copy
    /// of a rule, which is the distinction `saving_capacities` was fixed over.
    edges: Vec<karakuri_engine::set::Edge>,
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
    actions: Vec<karakuri_midi::Action>,
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
        let mut deck = build_deck(
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
                match mcp::serve(port, slots, self.args.store.clone(), self.args.watch) {
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
            store_root: self.args.store.clone(),
            save_tx,
            saves,
            saves_in_flight: 0,
            mcp,
            actions: Vec::new(),
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
    /// calls a key press makes.
    ///
    /// The whole of the MIDI connection, and it is one match on purpose: every
    /// arm below ends in a method the keyboard already reaches, so a surface
    /// can do nothing a key cannot and a session recorded from one replays with
    /// neither attached. A control added to one and not the other does not
    /// compile.
    ///
    /// Before the tick, so a fader move lands on the frame it arrived for
    /// rather than the one after — the same placement `run_demo` has, and for
    /// the same reason.
    fn run_surface(&mut self) {
        let Some(surface) = &mut self.midi else {
            return;
        };
        // Into the owned scratch, then out of `self`'s borrow, so the arms
        // below can call `&mut self` methods. Nothing allocates: both vectors
        // are reused and `take` clears rather than replaces.
        let mut actions = std::mem::take(&mut self.actions);
        // The slot check is the router's — it is where the "say it once"
        // machinery already is, and once per *message* would be a blocking
        // write per message on this thread. See `crate::midi`.
        surface.take(self.deck.slot_count(), &mut actions);
        for action in &actions {
            match *action {
                Action::Gain { slot, value } => self.set_gain(usize::from(slot), value),
                Action::Opacity { slot, value } => self.set_opacity(usize::from(slot), value),
                Action::Exposure { value } => self.set_exposure(value),
                Action::ToggleOnAir { slot } => self.toggle_on_air(usize::from(slot)),
                Action::TogglePriming { slot } => self.toggle_priming(usize::from(slot)),
                Action::CycleBlend { slot } => self.cycle_blend(usize::from(slot)),
                Action::Preview { slot } => self.show(slot.map(usize::from)),
                Action::Tap => self.tap(),
            }
        }
        self.actions = actions;
    }

    /// **What a model has asked for since the last frame.**
    ///
    /// Beside [`Live::run_surface`] and on the same terms: a surface is polled
    /// at the top of a frame and every request it produces ends in the method a
    /// key press ends in. That is what makes `--mcp` a third pair of hands
    /// rather than a second way to do anything.
    ///
    /// **Collected out of the borrow before any of it is acted on**, exactly as
    /// the MIDI actions are, because every arm below takes `&mut self`. Nothing
    /// is allocated on a frame that was asked for nothing: collecting an empty
    /// iterator makes no allocation.
    ///
    /// Here rather than beside the swap drain below `frame::compose`, which was
    /// the other candidate: a frame that finds no surface to draw on returns
    /// early, and a client asking to keep what is playing should not be waiting
    /// on a swapchain. Nothing a request reaches needs the GPU.
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
        for request in asked {
            self.save_set(request.slot, request.id, Some(request.reply));
        }
    }

    /// A key press. Returns true if it was a request to quit.
    ///
    /// Every branch prints what it did. There is no on-screen UI, and a control
    /// that changes something invisible — a gain on an off-air slot, an
    /// operator on a dark frame — is indistinguishable from a control that is
    /// broken.
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
                'v' => self.cycle_preview(),
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
                'k' => self.save_set(self.focus, None, None),
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

    /// On air and off again, for a named slot. Split from the key so a control
    /// surface can reach a slot its hands are not focused on — a fader bank
    /// has one strip per slot and no notion of focus at all.
    fn toggle_on_air(&mut self, slot: usize) {
        let t = self.deck.slot(slot).set().time();
        match self.deck.residency(slot) {
            Residency::Live => {
                self.record(mix::residency_record(slot, Residency::Allocated));
                eprintln!("slot {slot} off air — allocated, holding t {t:.2}s");
            }
            // Priming is warming out of sight and is still off air, so space
            // does the same thing to it: puts it on, at whatever `t` it has
            // warmed to.
            Residency::Allocated | Residency::Priming => {
                self.record(mix::residency_record(slot, Residency::Live));
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
        self.record(mix::residency_record(slot, want));
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
    /// key below, the MCP tool, and whatever M5 builds — for the same reason
    /// every mix control ends in one method. **This is the only save path**,
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
    fn save_set(&mut self, slot: usize, id: Option<String>, reply: Option<mcp::Reply>) {
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
        let id = accepted_save(slot, id, &sources, &self.store_root, reply.as_ref());
        let values = playing_values(self.deck.slot(slot).set(), &self.edges);
        let save = Save {
            slot,
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
            let (slot, id) = (save.slot, save.id.clone());
            let outcome = save.run();
            // **Carried back rather than answered from here.** This thread
            // knows the outcome and could send it, and that would be a second
            // place a save is reported from: the record is written at the frame
            // the outcome lands, and a model told anything else than what that
            // frame decided would be reading a different story from the stream.
            let _ = tx.send(Saved {
                slot,
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
    /// above every early return** — not beside the swap-event drain, which is
    /// where it used to be. See the comment at the head of [`Live::frame`]: a
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
            id,
            outcome,
            reply,
        } = saved;
        self.saves_in_flight = self.saves_in_flight.saturating_sub(1);
        let said = match outcome {
            Ok(()) => {
                let said =
                    format!("slot {slot}: saved as set `{id}` — load it with `--load-set {id}`");
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

        let bpm = self.deck.signals().oscillator().bpm();
        let engaged = Transport::engaged(next, bpm);
        self.record(mix::transport_record(slot, &engaged));
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
        let mut moved = *transport;
        moved.scrub(beats);
        self.record(mix::transport_record(slot, &moved));
        eprintln!(
            "slot {slot} scrub {:+.2} beats",
            self.deck.transport(slot).offset_beats()
        );
    }

    /// A tap on the beat. Authoritative — a performer tapping is stating where
    /// the beat is, not offering evidence — and it goes onto the oscillator
    /// through the same `tempo` record a tracked correction does.
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
        self.record(mix::gain_record(slot, clamp_gain(gain)));
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
        self.record(mix::opacity_record(slot, opacity));
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
    /// one in the dark.
    fn fade(&mut self, to: f32) {
        let slot = self.focus;
        self.fade_slot(slot, to);
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
    fn crossfade(&mut self) {
        let from = self.focus;
        let to = (from + 1) % self.deck.slot_count();
        if to == from {
            eprintln!("crossfade needs somewhere to go — this deck holds one slot");
            return;
        }
        self.record(mix::opacity_record(to, 0.0));
        if self.deck.residency(to) != Residency::Live {
            self.record(mix::residency_record(to, Residency::Live));
        }
        self.fade_slot(from, 0.0);
        self.fade_slot(to, 1.0);
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
    /// Under `add` the same gesture is a wipe *on* rather than a wipe *over*,
    /// which is a different picture and a legitimate one — so the mode is left
    /// wherever the operator had it, and `over` is only forced when the slot
    /// was still at the default. That way `m` in front of `c` means something.
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
        let mask = Mask::new(self.mask_kind, self.mask_angle, 0.0, MASK_SOFTNESS);
        self.record(mix::mask_record(over, mask));
        self.record(mix::opacity_record(over, 1.0));
        if self.deck.blend(over) == Blend::Add {
            self.record(mix::blend_record(over, Blend::Over));
        }
        if self.deck.residency(over) != Residency::Live {
            self.record(mix::residency_record(over, Residency::Live));
        }
        let now = self.deck.signals().oscillator().beats();
        let start = karakuri_engine::transition::quantise(now, self.quantum);
        self.record(mix::transition_record(
            over,
            karakuri_engine::transition::Control::MaskPosition,
            1.0,
            start,
            self.fade_beats,
            FADE_CURVE,
        ));
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
    /// diagonals, and an iris. A dial for the angle is M5's, where there is
    /// somewhere to see it.
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
    /// statement and wants its own vocabulary — `Record::Preview` carries
    /// `null` for "the mix" and is the shape it would take.
    ///
    /// **What it chooses is kept.** A live save reads the fold off the Set —
    /// see `playing_values` — so `k` after a press writes `{"t":"merge",
    /// "live":N}` and loading that file back comes up on renderer N.
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
        let now = self.deck.signals().oscillator().beats();
        let start = karakuri_engine::transition::quantise(now, self.quantum);
        self.record(mix::select_record(slot, next, start));
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

    /// One scheduled move on a slot's fader, through the record.
    ///
    /// The start is resolved **here**, once, against the grid as it stands: the
    /// record carries an absolute beat count, because "at the next bar" is a
    /// different instant depending on when it is read and a beat count is the
    /// same one on every run.
    fn fade_slot(&mut self, slot: usize, to: f32) {
        let now = self.deck.signals().oscillator().beats();
        let start = karakuri_engine::transition::quantise(now, self.quantum);
        self.record(mix::transition_record(
            slot,
            karakuri_engine::transition::Control::Opacity,
            to,
            start,
            self.fade_beats,
            FADE_CURVE,
        ));
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

    /// **Cycle what the output is showing**: the mix, then each slot in turn,
    /// then the mix again.
    ///
    /// Auditioning is a prerequisite rather than a
    /// convenience — choosing between candidates cannot be done blind. Every
    /// slot is offered whatever its residency, because an off-air slot is
    /// exactly the one worth looking at: an Allocated one shows the still it
    /// stopped at and a Priming one shows what it is warming into.
    ///
    /// The printed line says what is being shown *and* what the mix is doing
    /// without it, because the one thing an operator can lose track of here is
    /// which of the two they are looking at — a previewed slot that happens to
    /// be Live and alone in the mix is the same picture either way.
    fn cycle_preview(&mut self) {
        let count = self.deck.slot_count();
        let next = match self.deck.preview() {
            None => Some(0),
            Some(slot) if slot + 1 < count => Some(slot + 1),
            Some(_) => None,
        };
        self.show(next);
    }

    /// Show one slot, or the mix. Split from the cycle so a control surface can
    /// select one directly: a surface has a pad per slot, and reaching slot 3
    /// through three presses is a keyboard's compromise rather than a
    /// surface's.
    fn show(&mut self, slot: Option<usize>) {
        self.record(mix::preview_record(slot));
        match self.deck.preview() {
            Some(slot) => eprintln!(
                "preview slot {slot} — {}, gain {:.2}, t {:.2}s (the mix is not being shown)",
                residency_name(self.deck.residency(slot), self.deck.is_parked(slot)),
                self.deck.gain(slot),
                self.deck.slot(slot).set().time()
            ),
            None => eprintln!(
                "preview off — showing the mix of {} live",
                self.deck.live_slots()
            ),
        }
    }

    /// Cycle the focused slot's blend mode. No refusals here — unlike sync,
    /// every mode is available to every slot, because a blend mode is a
    /// question about pixels and not about what the material can do.
    fn cycle_blend(&mut self, slot: usize) {
        let current = self.deck.blend(slot);
        let at = Blend::ALL.iter().position(|b| *b == current).unwrap_or(0);
        let next = Blend::ALL[(at + 1) % Blend::ALL.len()];
        self.record(mix::blend_record(slot, next));
        eprintln!(
            "slot {slot} blend {} — gain {:.2}, opacity {:.2}",
            self.deck.blend(slot).name(),
            self.deck.gain(slot),
            self.deck.opacity(slot)
        );
    }

    fn cycle_tonemap(&mut self) {
        let look = Look {
            op: next_tonemap(self.look.op),
            ..self.look
        };
        self.record(mix::look_record(&look));
        eprintln!(
            "tonemap {} (exposure {:.2})",
            self.look.name(),
            self.look.exposure
        );
    }

    fn set_exposure(&mut self, exposure: f32) {
        let look = Look {
            exposure: clamp_exposure(exposure),
            ..self.look
        };
        self.record(mix::look_record(&look));
        eprintln!("exposure {:.3} ({})", self.look.exposure, self.look.name());
    }

    /// **Every mix change goes through here, and here goes through a record.**
    ///
    /// Built, decoded, and only then applied — so what drives the deck is what
    /// a replay would decode from a session stream, rather than a second path
    /// that happens to agree with it today. `audio.rs` does the same thing with
    /// the two records it emits; see `mix.rs` for the whole argument.
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

    fn record(&mut self, record: karakuri_store::record::Record) {
        if let Some(recorder) = &mut self.recorder {
            // Cloned, which allocates — and this is a key press rather than a
            // frame, so it is the one place in the record path where that is
            // allowed. The render-thread rule is about what a frame does.
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
            mix::Change::Preview { slot } => self.deck.set_preview(slot),
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
                offset_beats,
            } => {
                // The refusal is reported and nothing moves. It cannot happen
                // from a key press — `cycle_sync` only offers modes the Set
                // allows — but a session recorded against one Set and replayed
                // against another is exactly where it can, and a slot silently
                // left free would be a performance replayed wrong.
                if let Err(refusal) = self
                    .deck
                    .set_transport(slot, sync, anchor_bpm, offset_beats)
                {
                    eprintln!("slot {slot}: {} sync refused — {refusal}", sync.name());
                }
            }
        }
    }

    fn frame(&mut self) {
        self.run_demo();
        self.run_surface();
        // **Both halves of the save path, and both above every early return
        // below.** Taking the request here is [`Live::run_requests`]'s own
        // reasoning: a client asking to keep what is playing should not be
        // waiting on a swapchain, and nothing a request reaches needs the GPU.
        // The drain used to sit under `frame::compose`, which implemented half
        // of that and quietly withheld the other: a window latched to
        // `Skip::Fault`, or returning `Outdated` every frame, returns before it
        // — so the request was taken, the save thread wrote the file, and the
        // client waited out `SAVE_REPLY` to be told the outcome was neither
        // success nor failure about a save that had already landed. The
        // terminal never said "saved as set X" either, and the `save` record
        // was withheld from the stream until the run quit. A save's outcome has
        // nothing to do with whether there is a surface to draw on, which is
        // exactly what the two calls being here rather than there says.
        self.run_requests();
        self.finished_saves();

        // **The ordering that used to be a comment is now the shape of the
        // call.** A `tick` is a promise that the deck advanced by that many
        // steps, so nothing about a frame may be measured or recorded until
        // there is somewhere to draw it; `frame::compose` calls the closure
        // below only after its sink has a target, so there is no order left to
        // get wrong. It used to read the clock, write the `tick`, measure the
        // audio, and *then* find out the swapchain had nothing — and `Outdated`
        // arrives on every resize, so resizing during a recorded session made
        // the replay diverge from the performance.
        //
        // The clock not being read on an abandoned frame is what makes the
        // skipped interval *survive*: `Clock::last` moves only in `steps`, so
        // the time this frame did not use is counted by the next one. Up to
        // `MAX_STEPS`, which is the anti-spiral clamp and applies to any long
        // gap however it arose.
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
        let outcome = frame::compose(gpu, deck, present, sink, |deck| {
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
        });

        match outcome {
            Ok(frame::Outcome::Drawn) => {}
            // Ordinary: the swapchain is being remade and the next frame asks
            // again. Nothing was committed, so there is nothing to undo.
            Ok(frame::Outcome::Skipped(frame::Skip::Transient)) => return,
            // Printed unconditionally, because a `Fault` is already at most
            // one per condition: the sink latches it — see `frame::Skip`. The
            // latch used to be here, which meant the message was *built* every
            // frame and thrown away, an allocation on the frame path for as
            // long as the window stayed broken.
            Ok(frame::Outcome::Skipped(frame::Skip::Fault(why))) => {
                eprintln!("surface: {why}");
                return;
            }
            Err(e) => {
                eprintln!("surface: {e}");
                return;
            }
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
        // **What the output is showing, when it is not the mix.** First on the
        // line and not tucked in a column, because it is the one piece of state
        // that changes what every other number on the line is *about*: the
        // levels below are still per slot, but the picture is one of them.
        if let Some(slot) = self.deck.preview() {
            let _ = write!(self.status, "PVW{slot} ");
        }
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
                        if transport.offset_beats() == 0.0 {
                            String::new()
                        } else {
                            format!("{:+.2}", transport.offset_beats())
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
            self.look.name(),
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
    use karakuri_signal::NoiseKind;

    fn parse(args: &[&str]) -> Result<Args, String> {
        match parse_args_from(args.iter().map(|s| s.to_string())) {
            Ok(ParseOutcome::Run(args)) => Ok(*args),
            Ok(ParseOutcome::Help) => panic!("expected Args, got --help"),
            Ok(ParseOutcome::ListSets(_)) => panic!("expected Args, got --list-sets"),
            Ok(ParseOutcome::Bundle { .. }) => panic!("expected Args, got --bundle"),
            Ok(ParseOutcome::Unbundle { .. }) => panic!("expected Args, got --unbundle"),
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
                Ok(ParseOutcome::Bundle { .. }) | Ok(ParseOutcome::Unbundle { .. }) => {
                    panic!("{spelling:?} came back as a bundle")
                }
                Err(e) => panic!("{spelling:?} was refused: {e}"),
            }
        }
    }

    /// **Neither `--bundle` nor `--unbundle` is a run**, and the type says so
    /// for [`ParseOutcome::ListSets`]'s reason: there is no [`Args`] on either
    /// path, so the compile, the deck, the window and the adapter request are
    /// unreachable rather than merely skipped. `--unbundle` does reach the
    /// checker — that is how a metadata card gets written — and the check pass
    /// needs no device.
    ///
    /// **`--store` on either side of each**, because an operator types the
    /// flags in whatever order they think of them, and a bundle read out of the
    /// default store when `--store` was given would be a bundle of the wrong
    /// library.
    #[test]
    fn bundle_and_unbundle_print_and_are_never_a_run() {
        for spelling in [
            vec!["--bundle", "night01", "--store", "/tmp/library"],
            vec!["--store", "/tmp/library", "--bundle", "night01"],
        ] {
            match parse_args_from(spelling.iter().map(|s| s.to_string())) {
                Ok(ParseOutcome::Bundle { store, id }) => {
                    assert_eq!(store, PathBuf::from("/tmp/library"), "{spelling:?}");
                    assert_eq!(id, "night01", "{spelling:?}");
                }
                Ok(ParseOutcome::Run(_)) => panic!(
                    "{spelling:?} came back as a run: writing a bundle to stdout would then \
                     compile material and open a window to do it"
                ),
                other => panic!(
                    "{spelling:?} came back as something else: {}",
                    named(&other)
                ),
            }
        }
        for spelling in [
            vec!["--unbundle", "sent.ndjson", "--store", "/tmp/library"],
            vec!["--store", "/tmp/library", "--unbundle", "sent.ndjson"],
        ] {
            match parse_args_from(spelling.iter().map(|s| s.to_string())) {
                Ok(ParseOutcome::Unbundle { store, file }) => {
                    assert_eq!(store, PathBuf::from("/tmp/library"), "{spelling:?}");
                    assert_eq!(file, PathBuf::from("sent.ndjson"), "{spelling:?}");
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
            Ok(ParseOutcome::Bundle { .. }) => "--bundle".to_string(),
            Ok(ParseOutcome::Unbundle { .. }) => "--unbundle".to_string(),
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
                .open(root.join("sets").join(format!("{id}.set.ndjson")))
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

    /// **A demonstration brings the scene it is about.** `--demo lines` cycles
    /// the preview between the mix and each slot, and on a one-slot deck that
    /// shows the same picture twice — the demonstration would run, look like it
    /// worked, and demonstrate nothing.
    ///
    /// **Two slots, therefore, even though a stack would fit in one.** This was
    /// briefly rewritten to a single slot holding both renderers, on the
    /// grounds that it is the shape the milestone made possible — which broke
    /// it, because `DEMO_LINES_SCRIPT` presses the preview key three times and
    /// a preview isolates a slot rather than a renderer. Slot 0 carries the
    /// stack, which is what the milestone actually buys here: the same two
    /// draws, over one simulation instead of two.
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
            "the preview has to have more than one slot to cycle between"
        );
        assert_eq!(
            DEMO_LINES_SCRIPT.len(),
            args.sets.len() + 1,
            "the script presses the preview key once per slot and once to return \
             to the mix; a deck of a different size leaves it out of phase"
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
    /// all*.
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
    /// `docs/ir-spec.md`'s metadata section opened with "nothing writes or reads
    /// one" for the whole of M1 to M3, so every claim it makes was unenforced
    /// prose. This is the half a compile pass can produce, pinned against a
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
        // 33 characters, one range, two named keys — rather than a round number
        // under them, because a control surface is small enough that losing one
        // key is news and the scan going quiet is the thing being guarded
        // against. Three of them because they fail apart: a signature change
        // gives no arms at all, a broken literal reader gives named arms and no
        // characters, and a `NamedKey` renamed away gives characters and no
        // named ones. Raise them when a key is added; lowering one is a claim
        // that a control was deliberately removed.
        assert!(
            chars >= 33,
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

    /// **Both halves of the save path sit above every early return in
    /// [`Live::frame`]**, which is the whole of what makes an answer to a
    /// waiting client independent of there being a swapchain.
    ///
    /// [`Live::run_requests`] argues this for itself: a frame that finds no
    /// surface to draw on returns early, and a client asking to keep what is
    /// playing should not be waiting on one. The *outcome* drain used to sit
    /// below `frame::compose`, past three returns, so the argument was made and
    /// then half applied. On a window latched to `frame::Skip::Fault`, or
    /// returning `Outdated` every frame, the request was taken and the save
    /// thread wrote the file successfully — and the client waited out
    /// `mcp::SAVE_REPLY` to be told the outcome was neither success nor failure
    /// about a save already on disk, the terminal never said "saved as set X",
    /// and the `save` record promised for every save that reached
    /// the disk was withheld from the stream until the run quit.
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
    fn a_frame_attends_to_its_saves_before_it_can_return_early() {
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

        let first_return = code
            .find("return")
            .expect("`Live::frame` no longer returns early, and this test is about when it does");
        for call in ["self.run_requests();", "self.finished_saves();"] {
            let at = code
                .find(call)
                .unwrap_or_else(|| panic!("`Live::frame` no longer calls `{call}`"));
            assert!(
                at < first_return,
                "`{call}` is below an early return in `Live::frame`: a frame with nowhere \
                 to draw takes saves and never answers for them"
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
            set.write_param(&ParamWrite::everywhere("radius", 2.6));

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
        /// path, an `--mcp` write with no watcher behind it — which `mcp.rs` says
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
