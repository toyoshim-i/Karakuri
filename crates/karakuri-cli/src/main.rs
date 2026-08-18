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
mod midi;
mod mix;
mod render;
mod scratch;
mod session;
mod tempo_source;
mod setfile;
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
    Signals,
    TonemapOp, DEFAULT_BUDGET_MS,
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
    (MaskKind::Linear, std::f32::consts::FRAC_PI_2, "linear, bottom to top"),
    (MaskKind::Linear, std::f32::consts::FRAC_PI_4, "linear, diagonal"),
    (MaskKind::Linear, -std::f32::consts::FRAC_PI_4, "linear, the other diagonal"),
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
    fn deck(self) -> Vec<(PathBuf, Vec<PathBuf>)> {
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
                    "examples/drift_shell.kir".into(),
                    vec![
                        "examples/soft_points.kir".into(),
                        "examples/drift_streaks.kir".into(),
                    ],
                ),
                (
                    "examples/drift_shell.kir".into(),
                    vec!["examples/drift_streaks.kir".into()],
                ),
            ],
        }
    }
}

/// Where the store lives when nothing says otherwise. A directory in the
/// working tree rather than under `$HOME`: a session's material belongs beside
/// the session, and a global store shared by every run is a decision an
/// operator should make rather than inherit.
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
  --set L1.kir,L4.kir   name one slot. Repeat for more slots. Every path after
                        the first is sorted by the `kind` it declares: an L2
                        deforms the geometry, an L4 draws it. Order within a
                        kind is the order given
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
  --capacity N          elements per Set (default 262144)
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
  --save-set ID         put both `.kir` files in the store, write the material
                        as a Set file, and stop. The flags that were given —
                        capacity, params, binds, bpm — are what it records
  --load-set ID         take the material from a Set file rather than from two
                        paths and the flags. Anything the file could not carry
                        is printed rather than dropped in silence
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
  s          print the status line now
  h          print these bindings
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
    TONEMAPS.iter().copied().find(|op| op_wire_name(*op) == name)
}

/// Every wire spelling, for an error message that says what was available.
pub fn op_wire_names() -> String {
    TONEMAPS
        .iter()
        .map(|op| op_wire_name(*op))
        .collect::<Vec<_>>()
        .join(", ")
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
    /// The session tempo. There is **no tempo record** in the v0.2 vocabulary,
    /// so unlike `--bind` this flag has nothing to map onto yet; it is here
    /// because a binding to `beat` is meaningless at a tempo nobody can set.
    bpm: f32,
    /// One deck slot per entry, in composite order: the L1 that simulates, and
    /// the renderers drawn over it in list order — see `Set::build_many`. One
    /// renderer is the ordinary case; several is one simulation drawn several
    /// ways, which costs a draw pass apiece and no extra memory.
    sets: Vec<(PathBuf, Vec<PathBuf>)>,
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
    /// What a Set file said about the seed and the camera, when one was loaded.
    /// Not flags: there is no `--seed` and no `--camera`, and inventing two so
    /// that a file could be read would be adding surface to carry a value
    /// rather than to be used.
    from_set: Option<(Option<u32>, Option<karakuri_engine::camera::Orbit>)>,
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
                layer = Some(
                    layer_named(v).ok_or_else(|| {
                        bad(&format!("`layer={v}` — expected L1, L2, L3 or L4"))
                    })?,
                )
            }
            "index" => {
                index = Some(v.parse::<u32>().map_err(|_| {
                    bad(&format!("`index={v}` — expected a node number, 0 for the first"))
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
                noise_octaves = Some(
                    v.parse::<u32>().map_err(|_| {
                        bad(&format!("`noise.octaves={v}` — expected a whole number"))
                    })?,
                );
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
    // cannot differ between a command line and a Set file. `docs/roadmap.md`
    // recorded that debt against the decoder; this is it paid by having one
    // rule rather than two copies of it.
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
        layer: match layer {
            karakuri_ir::Kind::L1 => Layer::L1,
            karakuri_ir::Kind::L2 => Layer::L2,
            karakuri_ir::Kind::L3 => Layer::L3,
            karakuri_ir::Kind::L4 => Layer::L4,
            // `--bind` parses its layer from a fixed list that has no `Field`,
            // so this is unreachable by construction rather than by argument —
            // and it stays unreachable until the record format grows one.
            karakuri_ir::Kind::Field => unreachable!("`--bind` has no `Field` layer to parse"),
        },
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
fn layer_named(name: &str) -> Option<karakuri_ir::Kind> {
    Some(match name {
        "L1" => karakuri_ir::Kind::L1,
        "L2" => karakuri_ir::Kind::L2,
        "L3" => karakuri_ir::Kind::L3,
        "L4" => karakuri_ir::Kind::L4,
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
        format!("`--publish {value}` — expected `name=key[LOW..HIGH]` or `name=L4:0:key[LOW..HIGH]`")
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
                        if !l1.is_empty() && !l4s.is_empty() && l4s.iter().all(|p| !p.is_empty()) =>
                    {
                        args_out.sets.push((
                            PathBuf::from(*l1),
                            l4s.iter().map(PathBuf::from).collect(),
                        ))
                    }
                    _ => {
                        return Err(format!(
                            "`--set {value}` — expected `L1.kir,L4.kir`, or \
                             `L1.kir,L4.kir,L4.kir` for several renderers over one geometry"
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
                args_out.look.op = parse_op(&value).ok_or_else(|| {
                    format!("`--tonemap {value}` — expected {}", op_wire_names())
                })?;
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
                    _ => {
                        return Err(format!(
                            "`--exposure {value}` — expected a positive number"
                        ))
                    }
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
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`"))
            }
            _ => positional.push(PathBuf::from(arg)),
        }
    }

    // The bare positional pair still means what it always meant, and now it is
    // simply the last slot: `--set a,b c.kir d.kir` is a deck of two.
    match positional.len() {
        0 => {}
        2 => args_out
            .sets
            .push((positional[0].clone(), vec![positional[1].clone()])),
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
                "examples/drift_shell.kir".into(),
                vec!["examples/soft_points.kir".into()],
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
        return Err("--load-set and --save-set in one run: it would rewrite what it just read"
            .to_string());
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
    l1.capacity.map_or(args.capacity, |declared| declared.default)
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

/// One deck slot's material, sorted into the chain a `Set` is built from: the
/// procedure that simulates, the deformations between, and the renderers drawn
/// over the result.
///
/// **The sorting happens here rather than on the command line**, because every
/// `.kir` declares its own `kind` — so `--set` is one comma-separated list and
/// the loader reads which is which off the files. Order *within* a kind is list
/// order, which is chain order for L2s and draw order for L4s.
struct Material {
    l1: karakuri_ir::typed::Checked,
    l2s: Vec<karakuri_ir::typed::Checked>,
    /// The camera, or `None` for the built-in orbit. At most one per slot.
    l3: Option<karakuri_ir::typed::Checked>,
    l4s: Vec<karakuri_ir::typed::Checked>,
}

/// **Render a recorded session.** The material comes from the stream's head and
/// every frame advances by the `tick` that was recorded, so nothing here reads
/// a clock — which is the whole claim: a replay and the run it came from are
/// the same sequence of frames.
///
/// Offscreen only. A window would add a clock back at the one place a replay
/// must not have one: `RedrawRequested` arrives when the display says so, and a
/// replay's frames belong to the stream.
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
    // Said rather than dropped, on the same terms as everything else here: a
    // session that ended between frames recorded what the operator last did,
    // and nothing renders it because there is no frame it belongs to.
    if !stream.trailing.is_empty() {
        eprintln!(
            "  {} record{} after the last tick belong to no frame and are not replayed",
            stream.trailing.len(),
            if stream.trailing.len() == 1 { "" } else { "s" }
        );
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

    let mut set = build(
        &gpu,
        &loaded.l1,
        // **A Set file carries neither a chain nor a camera yet.** It records
        // an L1 and its renderers, so a replay of one draws them from the
        // built-in orbit — the same gap L2 has, and the same fix will close
        // both. `--set` is where a chain and a camera are spelled today.
        &[],
        None,
        &loaded.l4s,
        // **A Set file does not record a layering**, on the same terms it
        // records neither a chain nor a camera: it names an L1 and its
        // renderers. Overdraw is what every Set was before an L5 could be
        // nested, so it is what a file that cannot say reads as.
        karakuri_engine::set::Layering::Overdraw,
        loaded.capacity.unwrap_or(args.capacity),
        &loaded.params,
        &loaded.bindings,
        // **A Set file does not record an interface yet**, on the same terms it
        // records neither a chain nor a camera nor a layering: it names an L1,
        // its renderers, their values and their bindings. Publishing nothing is
        // publishing everything, so a replay shows the whole console — which is
        // the safe direction, since an interface is about attention.
        &[],
        loaded.seed.unwrap_or_else(|| seed_for(0)),
        loaded.camera,
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
                apply_replayed(deck, &mut look, record);
            }
            for slot in changed {
                match rebuild(&gpu, &store, &playing[slot], args, slot) {
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
        &l1,
        // Artifacts recorded by a session, which stores an L1 and its
        // renderers — see the note at the other `build` call site.
        &[],
        None,
        &l4s,
        karakuri_engine::set::Layering::Overdraw,
        args.capacity,
        &args.overrides,
        &args.bindings,
        &[],
        seed_for(slot),
        None,
    ))
}

/// One record from a session, applied to a replaying deck.
///
/// Deliberately the same decoders the live path uses — `mix::change` and
/// `audio::apply_tempo` — because that is the whole point of the arrangement:
/// what drove the engine live and what drives it on replay are the same
/// function, so they cannot come apart.
fn apply_replayed(
    deck: &mut Deck,
    look: &mut Look,
    record: &karakuri_store::record::Record,
) {
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
    store: &karakuri_store::store::Store,
    id: &str,
) -> Vec<karakuri_store::ndjson::Line> {
    if args.sets.len() > 1 {
        eprintln!(
            "  only slot 0's material is in the session's head — a session stream cannot \
             say what a deck held, so the other {} will not replay",
            args.sets.len() - 1
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
    let Some((l1, l4s)) = args.sets.first() else {
        eprintln!("karakuri-cli: nothing to record — no Set to put at the session's head");
        std::process::exit(2);
    };
    let material = format!("{id}-material");
    let camera = karakuri_engine::camera::Orbit::default();
    if let Err(e) = setfile::save(
        store,
        &material,
        setfile::Saving {
            l1_path: l1,
            l4_paths: l4s,
            capacity: args.capacity,
            params: &args.overrides,
            bindings: &args.bindings,
            camera: &camera,
            seed: seed_for(0),
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

fn save_set(args: &Args, id: &str) {
    let store = open_store(args);
    let Some((l1, l4s)) = args.sets.first() else {
        eprintln!("karakuri-cli: --save-set needs a `.kir` pair to save");
        std::process::exit(1);
    };
    if args.sets.len() > 1 {
        eprintln!(
            "  only slot 0 is saved: a Set file describes one Set, and which Sets a deck              is holding belongs to a session"
        );
    }
    let camera = karakuri_engine::camera::Orbit::default();
    match setfile::save(
        &store,
        id,
        setfile::Saving {
            l1_path: l1,
            l4_paths: l4s,
            capacity: args.capacity,
            params: &args.overrides,
            bindings: &args.bindings,
            camera: &camera,
            seed: seed_for(0),
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
    if let Some(capacity) = loaded.capacity {
        args.capacity = capacity;
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
    args.from_set = Some((loaded.seed, loaded.camera));
    loaded
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
    // Whether anything in this run can write a `.kir`. Named because two things
    // turn on it and they have to turn on the same one: what gets copied into
    // the scratch, and what gets compiled out of `args.sets` rather than out of
    // the Set file directly.
    let editable = args.watch || args.mcp.is_some();
    if editable {
        let root = args.store.clone();
        // A loaded Set names its procedures by hash and has no file anywhere,
        // so it is written into the scratch first and then joins `args.sets` as
        // ordinary material. Everything downstream — the compile, the watcher,
        // the MCP surface — then treats it exactly like a `--set` pair, which
        // is what makes a saved Set editable rather than only playable.
        if let Some(loaded) = &loaded {
            let placed: Result<Vec<PathBuf>, String> =
                std::iter::once((&loaded.l1.name, &loaded.l1_src))
                    .chain(loaded.l4s.iter().map(|c| &c.name).zip(loaded.l4_srcs.iter()))
                    .map(|(name, src)| scratch::place(&root, name, src))
                    .collect();
            match placed {
                Ok(paths) => args.sets.insert(0, (paths[0].clone(), paths[1..].to_vec())),
                Err(e) => {
                    eprintln!("karakuri-cli: {e}");
                    std::process::exit(1);
                }
            }
        }
        match scratch::materialise(&root, &mut args.sets) {
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
        history::seed(&shared, &args.sets);
        snapshots = Some(shared);
    }

    eprintln!("compiling:");
    let mut procs: Vec<Material> = Vec::new();
    // Only when it was *not* materialised into the scratch above. An editable
    // run compiles it out of `args.sets` with everything else, which is the
    // point: one path, so a loaded Set can be watched and rewritten.
    if let Some(loaded) = loaded.filter(|_| !editable) {
        eprintln!("  slot 0: set `{}`", loaded.id);
        procs.push(Material { l1: loaded.l1, l2s: Vec::new(), l3: None, l4s: loaded.l4s });
    }
    for (slot, (l1, rest)) in args.sets.iter().enumerate() {
        let named: Vec<String> = rest.iter().map(|p| p.display().to_string()).collect();
        eprintln!("  slot {slot}: {} + {}", l1.display(), named.join(" + "));
        let load = |path: &PathBuf| match compile::load(path) {
            Ok(checked) => checked,
            Err(report) => {
                eprintln!("{report}");
                std::process::exit(1);
            }
        };
        // **Sorted by the `kind` each file declares**, keeping list order within
        // a kind. A second L1 is refused rather than silently ignored: a slot
        // simulates with one geometry, and a run that quietly dropped the
        // second would be playing something nobody asked for.
        let mut l2s = Vec::new();
        let mut l3: Option<karakuri_ir::typed::Checked> = None;
        let mut l4s = Vec::new();
        for (path, checked) in rest.iter().zip(rest.iter().map(load)) {
            match checked.kind {
                karakuri_ir::Kind::L2 => l2s.push(checked),
                karakuri_ir::Kind::L4 => l4s.push(checked),
                // **One camera per slot**, refused rather than last-one-wins for
                // the same reason a second L1 is: a Set is a grouping around one
                // viewpoint, and a run that quietly dropped one of two would be
                // playing something nobody asked for. Two viewpoints composited
                // is a graph, which is what an L5 is for.
                karakuri_ir::Kind::L3 if l3.is_some() => {
                    eprintln!(
                        "slot {slot}: {} is a second L3 — a slot looks from one camera, and \
                         compositing two viewpoints is what an L5 is for",
                        path.display()
                    );
                    std::process::exit(1);
                }
                karakuri_ir::Kind::L3 => l3 = Some(checked),
                // **Refused rather than dropped**, on the same terms as a
                // second L1: the checker accepts a `kind Field` file and
                // nothing yet splices one into the procedures that evaluate it,
                // so a run that took the path and ignored it would draw a
                // picture nobody asked for.
                karakuri_ir::Kind::Field => {
                    eprintln!(
                        "slot {slot}: {} is a `kind Field`, which checks but which a Set \
                         cannot hold yet — a field lowers into whoever evaluates it and \
                         nothing does that today",
                        path.display()
                    );
                    std::process::exit(1);
                }
                karakuri_ir::Kind::L1 => {
                    eprintln!(
                        "slot {slot}: {} is an L1 and so is {} — a slot simulates with one \
                         geometry and draws it with as many renderers as it likes",
                        path.display(),
                        l1.display()
                    );
                    std::process::exit(1);
                }
            }
        }
        if l4s.is_empty() {
            eprintln!(
                "slot {slot}: nothing here draws — {} names no L4, and a Set with no \
                 renderer has no frame to give",
                l1.display()
            );
            std::process::exit(1);
        }
        procs.push(Material { l1: load(l1), l2s, l3, l4s });
    }

    // Saving is a one-shot: it writes what the flags say and stops, on the same
    // terms as `--render`. Running afterwards would leave an operator unsure
    // whether what they are watching is what was written.
    if let Some(id) = &args.save_set {
        save_set(&args, id);
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
            "  slot {slot}: {} live of {}",
            set.live_count(&gpu.device, &gpu.queue),
            set.capacity()
        );
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
    // Where a rebuilt procedure is stored and reported, when a session is
    // being recorded. `None` and no watcher touches a store.
    recording: Option<(
        std::sync::Arc<karakuri_store::store::Store>,
        std::sync::mpsc::Sender<watch::Built>,
    )>,
    // The run's edit history, shared by every slot's watcher and already
    // holding what the run started with. `None` for a run that cannot be
    // edited — see `history`.
    snapshots: Option<history::Shared>,
) -> Deck {
    let swaps = procs
        .iter()
        .enumerate()
        .map(|(slot, material)| {
            let (l1, l2s, l3, l4s) =
                (&material.l1, &material.l2s, material.l3.as_ref(), &material.l4s);
            let set = build(
                gpu,
                l1,
                l2s,
                l3,
                l4s,
                if args.merge.contains(&slot) {
                    karakuri_engine::set::Layering::Composite
                } else {
                    karakuri_engine::set::Layering::Overdraw
                },
                capacity_for(args, l1),
                &args.overrides,
                &args.bindings,
                &args.published,
                // A Set file's own seed when it named one, so a saved Set
                // reproduces rather than being re-salted by the slot it lands
                // in. It only ever applies to slot 0: `--load-set` fills that
                // slot and the rest come from `--set`.
                match (slot, args.from_set.as_ref()) {
                    (0, Some((Some(seed), _))) => *seed,
                    _ => seed_for(slot),
                },
                match (slot, args.from_set.as_ref()) {
                    (0, Some((_, camera))) => *camera,
                    _ => None,
                },
            );
            if watch {
                // One worker and one watcher per slot, over that slot's own
                // two files. That is what makes "the slot whose files changed"
                // the thing that rebuilds: no slot can see another's edit.
                HotSwap::new(
                    &gpu.device,
                    &gpu.queue,
                    set,
                    args.budget_ms,
                    {
                        let layering = if args.merge.contains(&slot) {
                            karakuri_engine::set::Layering::Composite
                        } else {
                            karakuri_engine::set::Layering::Overdraw
                        };
                        let watcher = watch::Watch::new(
                            slot,
                            args.sets[slot].0.clone(),
                            args.sets[slot].1.clone(),
                            layering,
                            capacity_for(args, l1),
                            seed_for(slot),
                            args.overrides.clone(),
                            args.published.clone(),
                            args.bindings.clone(),
                        );
                        // **Only when a session is being recorded.** Without
                        // one there is nothing to name and no store to name it
                        // in, and the watcher does no I/O it did not do before.
                        // The history is kept whether or not a session is
                        // being recorded: the two answer different questions —
                        // see `Watch::snapshots`. One `Shared` across every
                        // slot, because it is also what the launch-time seed
                        // wrote into.
                        let watcher = match &snapshots {
                            Some(shared) => watcher.snapshotting_to(shared.clone()),
                            None => watcher,
                        };
                        Box::new(match &recording {
                            Some((store, tx)) => {
                                watcher.recording_to(store.clone(), tx.clone())
                            }
                            None => watcher,
                        })
                    },
                )
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
    for binding in &args.bindings {
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
    l1: &karakuri_ir::typed::Checked,
    l2s: &[karakuri_ir::typed::Checked],
    l3: Option<&karakuri_ir::typed::Checked>,
    l4s: &[karakuri_ir::typed::Checked],
    layering: karakuri_engine::set::Layering,
    capacity: u32,
    overrides: &[ParamWrite],
    bindings: &[Binding],
    published: &[karakuri_engine::set::Published],
    seed: u32,
    camera: Option<karakuri_engine::camera::Orbit>,
) -> Set {
    let deform: Vec<&karakuri_ir::typed::Checked> = l2s.iter().collect();
    let draw: Vec<&karakuri_ir::typed::Checked> = l4s.iter().collect();
    match Set::build_many(&gpu.device, &gpu.queue, l1, &deform, l3, &draw, layering, capacity, seed)
    {
        Ok(mut set) => {
            // **The `camera` record, and only when nothing else produces one.**
            // An L3 writes the camera state every frame, so an orbit assigned
            // here would be overwritten before the first draw — silently, which
            // is the wrong way for two producers to meet.
            if let Some(camera) = camera.filter(|_| l3.is_none()) {
                set.camera = camera;
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
            for binding in bindings {
                let (layer, key) = (binding.layer, binding.key.clone());
                if !set.bind(binding.clone()) {
                    eprintln!("  no {layer:?} parameter named `{key}` to bind, ignoring");
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
    /// The procedure each slot is recorded as playing, and the one before it.
    /// The second is what a rollback restores, and the only way to name it: a
    /// rollback brings back a Set the stream never named again.
    playing: Vec<Option<(karakuri_store::hash::Hash, Vec<karakuri_store::hash::Hash>)>>,
    previous: Vec<Option<(karakuri_store::hash::Hash, Vec<karakuri_store::hash::Hash>)>>,
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
        let (rebuilds, rebuild_rx) = match &self.args.record_session {
            Some(_) => {
                let store = std::sync::Arc::new(open_store(&self.args));
                let (tx, rx) = std::sync::mpsc::channel();
                (Some((store, tx)), Some(rx))
            }
            None => (None, None),
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
                    l1.display(),
                    l4s.iter()
                        .map(|p| p.display().to_string())
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
                match audio::Audio::open(
                    selector,
                    self.args.latency_offset_ms,
                    DT,
                    self.args.bpm,
                ) {
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
            Some(selector) => {
                match midi::Surface::open(selector, self.args.midi_map.as_deref()) {
                    Ok(surface) => Some(surface),
                    Err(e) => {
                        eprintln!("karakuri-cli: {e}");
                        std::process::exit(2);
                    }
                }
            }
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
                let slots = mcp::Slots(self.args.sets.clone());
                match mcp::serve(port, slots, self.args.watch) {
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
                let head = session_head(&self.args, &store, id);
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
            playing: vec![None; slot_count],
            previous: vec![None; slot_count],
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
            eprintln!(
                "no slot {slot}: this deck holds slots 0-{}",
                self.deck.slot_count() - 1
            );
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
    /// Say what a slot is playing, now that it changed.
    ///
    /// `landed` is the build id when a swap went in, or `None` when one was
    /// rolled back — which is the case the pair kept in `previous` exists for.
    /// A rollback brings back a Set the stream will never name again, so the
    /// only way to say what came back is to have remembered it.
    ///
    /// **One thing this cannot carry.** The rollback restores the outgoing Set
    /// at the `t` it was parked at; a replay meeting these records builds
    /// afresh, so `t` restarts there. A swap *in* is documented to start cold
    /// and so replays exactly. Only a rollback differs, and a rollback means
    /// the candidate was over budget — an exceptional frame already.
    fn record_procedure(&mut self, slot: usize, landed: Option<u64>) {
        if self.recorder.is_none() {
            return;
        }
        // Drained here rather than per frame: the channel only has anything in
        // it when a build has just been requested, and this runs when one has
        // just landed.
        if let Some(rx) = &self.rebuilds {
            while let Ok(built) = rx.try_recv() {
                self.pending_builds.insert(built.id, built);
            }
        }

        let pair = match landed {
            Some(id) => {
                let Some(built) = self.pending_builds.remove(&id) else {
                    // The watcher could not store this build's source and said
                    // so at the time. Nothing to name.
                    return;
                };
                self.previous[slot] = self.playing[slot].take();
                self.playing[slot] = Some((built.l1, built.l4s));
                self.playing[slot].clone()
            }
            None => {
                let restored = self.previous[slot].take();
                self.playing[slot] = restored.clone();
                restored
            }
        };
        let Some((l1, l4s)) = pair else {
            // A rollback to the procedure the run started with, which the head
            // already names. Nothing changed that the stream does not say.
            return;
        };
        // One record per node: the L1, then each renderer in draw order. The
        // index is what says which renderer, and it is 0 for the first — so a
        // slot with one renderer writes exactly the two lines it always did.
        let named = std::iter::once((Layer::L1, 0, l1))
            .chain(l4s.into_iter().enumerate().map(|(i, h)| (Layer::L4, i as u32, h)));
        for (layer, index, hash) in named {
            self.record_only(karakuri_store::record::Record::Procedure {
                slot: slot as u8,
                layer,
                index,
                proc_hash: hash,
            });
        }
    }

    /// Push a record without applying it.
    ///
    /// The one place this is right: a `procedure` record *describes* a change
    /// the engine has already made, rather than asking for one. Everything else
    /// goes through `Live::record`, which applies what it wrote.
    fn record_only(&mut self, record: karakuri_store::record::Record) {
        if let Some(recorder) = &mut self.recorder {
            recorder.push(record);
        }
    }

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
            eprintln!("slot {slot}: {} is the only mode this material takes", next.name());
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
            None => eprintln!("no audio input — the latency offset only means something with --audio-in"),
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
    /// Auditioning, which `docs/roadmap.md` calls a prerequisite rather than a
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
            None => eprintln!("preview off — showing the mix of {} live", self.deck.live_slots()),
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
            // allowed. `README.md`'s invariant is about what a frame does.
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
                if let Err(refusal) =
                    self.deck
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
            self.record_procedure(slot, landed);
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
                if a.half_tempo_hint && a.estimate_confidence >= karakuri_audio::lock::GATE_CONFIDENCE
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
            Err(e) => Err(e),
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

    // -- midi ------------------------------------------------------------

    #[test]
    fn midi_is_off_unless_asked_for_and_takes_a_port_name() {
        assert_eq!(parse(&[]).expect("parses").midi_in, None);
        assert_eq!(parse(&[]).expect("parses").midi_map, None);
        assert_eq!(
            parse(&["--midi-in", "nanoKONTROL"]).expect("parses").midi_in,
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
        let message =
            parse(&["--midi-map", "surface.map"]).expect_err("a map with nothing to map");
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
        args.sets = vec![(l1, vec![l4])];
        args.store = dir.path().to_path_buf();

        let head = session_head(&args, &store, "a_set");
        assert!(!head.is_empty(), "the head describes nothing");

        // Read back the way `--replay` reads it, which is the whole claim: the
        // artifacts resolve out of the store and both slots are there.
        let loaded = setfile::from_lines(&store, "a_set", &head)
            .expect("the head a recording writes is a head a replay can load");
        assert_eq!(loaded.l1.kind, karakuri_ir::Kind::L1);
        assert_eq!(loaded.l4s[0].kind, karakuri_ir::Kind::L4);
        assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
    }

    #[test]
    fn no_arguments_is_the_default_pair() {
        let args = parse(&[]).expect("should parse");
        assert_eq!(
            args.sets,
            vec![(
                PathBuf::from("examples/drift_shell.kir"),
                vec![PathBuf::from("examples/soft_points.kir")],
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
                    PathBuf::from("examples/drift_shell.kir"),
                    vec![
                        PathBuf::from("examples/soft_points.kir"),
                        PathBuf::from("examples/drift_streaks.kir"),
                    ],
                ),
                (
                    PathBuf::from("examples/drift_shell.kir"),
                    vec![PathBuf::from("examples/drift_streaks.kir")],
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
            vec![(PathBuf::from("a.kir"), vec![PathBuf::from("b.kir")])]
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
                PathBuf::from("examples/drift_shell.kir"),
                vec![PathBuf::from("examples/soft_points.kir")],
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
            vec![(PathBuf::from("a.kir"), vec![PathBuf::from("b.kir")])]
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
                (PathBuf::from("a.kir"), vec![PathBuf::from("b.kir")]),
                (PathBuf::from("c.kir"), vec![PathBuf::from("d.kir")]),
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
        for bad in ["L4:exposure=2.5", "L4:x:exposure=2.5", "L9:0:exposure=2.5", ":0:e=1", "=2.5"] {
            let err = parse(&["--param", bad]).unwrap_err();
            assert!(err.contains("--param"), "`{bad}` was accepted or misreported: {err}");
        }
    }

    /// `--bind` needed no new grammar at all: its fields are the record's, so
    /// the address is one more field. Absent is a wildcard there too.
    #[test]
    fn a_bind_can_address_one_renderer_and_defaults_to_all_of_them() {
        let all = parse(&["--bind", "layer=L4,key=exposure,signal=beat,range=0..1"])
            .expect("should parse");
        assert_eq!(all.bindings[0].index, None, "a bind with no index is the layer's");

        let one = parse(&["--bind", "layer=L4,index=2,key=exposure,signal=beat,range=0..1"])
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
    /// is exactly what `docs/roadmap.md` said the command line should look like
    /// — *"no new syntax at all"*.
    #[test]
    fn set_with_three_paths_is_one_geometry_and_two_renderers() {
        let args = parse(&["--set", "a.kir,b.kir,c.kir"]).expect("should parse");
        assert_eq!(
            args.sets,
            vec![(
                PathBuf::from("a.kir"),
                vec![PathBuf::from("b.kir"), PathBuf::from("c.kir")],
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
        let plain = parse(&["--bind", "layer=L4,key=hue,signal=beat,range=0..1"])
            .expect("should parse");
        assert_eq!(plain.bindings[0].curve, Curve::Lin);
        assert_eq!(plain.bindings[0].noise, None, "only a noise signal gets one");

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
            ("layer=L4,key=hue,signal=beat,curv=lin,range=0..1", "unknown field `curv`"),
            ("layer=L4,key=hue,signal=beat,range=0..1,noise.rate=2", "needs `signal=noise`"),
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
        assert!(err.contains("beat"), "the refusal does not say what to use: {err}");

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
            vec!["--replay", "s", "--render", "out.png", "--size", "1920x1080"],
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
            vec!["--tempo-source", "helper", "--replay", "a", "--render", "out.png"],
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
            vec!["--replay", "a", "--render", "out.png", "--record-session", "s"],
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
        for flag in ["--render", "--seq", "--frames", "--size", "--set", "--exposure"] {
            let err = parse(&[flag]).unwrap_or_else(|| panic!("`{flag}` alone was accepted"));
            assert!(err.contains(flag) && err.contains("needs a value"), "{flag} -> {err}");
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
        assert!(err.contains("positive"), "rejected for the wrong reason: {err}");
    }
}
