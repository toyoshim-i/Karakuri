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
mod mix;
mod render;
mod watch;

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use karakuri_engine::binding::{Curve, CURVES, DEFAULT_BPM, NOISE_SIGNAL};
use karakuri_engine::deck::MAX_SLOTS;
use karakuri_engine::swap::Event;
use karakuri_engine::{
    Binding, Deck, Gpu, HotSwap, Present, Residency, Set, Signals, TonemapOp, DEFAULT_BUDGET_MS,
};
use karakuri_signal::{NoiseConfig, NoiseKind};
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

/// A fader floor: a negative gain would subtract one slot's light from
/// another's, which is a blend mode rather than a fader. Not ceilinged — the
/// pipeline is HDR and values above 1.0 are expected. Pure for the same
/// reason as [`clamp_exposure`].
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
  --set L1.kir,L4.kir   name one pair. Repeat for more slots
  L1.kir L4.kir         the same thing, positionally, for one pair. Given
                        alongside --set it becomes the last slot
  (nothing)             examples/drift_shell.kir + examples/soft_points.kir

options:
  --render FILE         render one frame offscreen and stop
  --seq DIR             render every frame to DIR/%05d.png and stop
  --frames N            how many simulation frames (default 240)
  --size WxH            output size (default 1280x720)
  --capacity N          elements per Set (default 262144)
  --param name=value    a uniform write, applied to every Set
  --bind FIELDS         attach a signal to a param, applied to every Set.
                        Comma-separated `field=value`, one per field of the
                        `bind` record:
                          layer=L1 key=turbulence signal=energy
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
  --tonemap OP          clamp | reinhard | aces | agx (default aces)
  --exposure V          output exposure, before the tone map (default 1.0)
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
  [ ]        focused slot gain down / up
  \\          focused slot gain back to 1.0
  t          cycle the tone map operator: clamp, Reinhard, ACES, AgX
  - =        output exposure down / up
  `          exposure back to 1.0
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
    overrides: Vec<(String, f32)>,
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
    /// One (L1, L4) pair per deck slot, in composite order.
    sets: Vec<(PathBuf, PathBuf)>,
    capacity: u32,
    render_to: Option<PathBuf>,
    seq_to: Option<PathBuf>,
    frames: u32,
    size: (u32, u32),
    /// Watch every pair and hot-swap the slot whose files changed. Off by
    /// default: a run that is not being edited should not carry a worker
    /// thread per slot and a watchdog it will never use.
    watch: bool,
    /// `--audio-in`. `None` is no device at all, which is not the same as a
    /// device that is silent: with no device the bus answers `energy` and the
    /// bands exactly as it did before audio existed, and the oscillator
    /// free-runs.
    audio_in: Option<String>,
    /// The unmeasurable half of the output lag, in milliseconds. See
    /// `audio::DEFAULT_LATENCY_OFFSET_MS`.
    latency_offset_ms: f32,
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
                layer = Some(match v {
                    "L1" => karakuri_ir::Kind::L1,
                    "L4" => karakuri_ir::Kind::L4,
                    // L2 and L3 are in the record's `Layer` and are M3's; a
                    // Set has no slot for one, so binding into it would
                    // silently do nothing.
                    _ => return Err(bad(&format!("`layer={v}` — expected L1 or L4"))),
                })
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

    // A tempo is not a `[0, 1]` signal. Steps 2 and 3 of a binding clamp the
    // sample into the unit range before mapping it, so `bpm` — which is 120,
    // not 0.42 — arrives as 1.0 and the param sits at the top of its range for
    // the whole run. `docs/ir-spec.md` records that; refusing it here is what
    // makes it visible, because from the outside a pinned binding and a
    // working one are the same number on a status line. `beat` and `bar` carry
    // the same tempo in the range a binding is defined over.
    if signal == "bpm" {
        return Err(bad(
            "`signal=bpm` — a tempo is not a [0, 1] signal, so the curve clamps it and this \
             binding would sit at the top of its range for the whole run; bind `beat` or \
             `bar` instead",
        ));
    }

    // `octaves` is `fbm`'s layer count and the other three kinds have no
    // layers. Refused rather than ignored, on the same terms as a `noise.*`
    // field on a binding that is not a noise binding: the alternative is an
    // operator watching a generator behave like one they did not name.
    if noise_octaves.is_some() && noise_kind != Some("fbm") {
        return Err(bad(&format!(
            "`noise.octaves` needs `noise.kind=fbm`, and this asks for `{}`",
            noise_kind.unwrap_or("perlin")
        )));
    }
    noise.kind = match noise_kind {
        Some("white") => NoiseKind::White,
        Some("value") => NoiseKind::Value,
        Some("fbm") => NoiseKind::Fbm {
            octaves: noise_octaves.unwrap_or(DEFAULT_OCTAVES),
        },
        // `perlin`, and the default when nothing named a kind — which are the
        // same generator, so they are the same arm.
        _ => NoiseKind::Perlin,
    };

    let binding = Binding::new(layer, key, signal.clone(), curve, range);
    if signal == NOISE_SIGNAL {
        Ok(binding.with_noise(noise))
    } else if noise_given {
        // Accepting this would leave the operator watching a parameter that
        // does not move and re-reading the noise fields to find out why.
        Err(bad(&format!(
            "`noise.*` needs `signal={NOISE_SIGNAL}`, and this binds `{signal}`"
        )))
    } else {
        Ok(binding)
    }
}

/// The octave count an `fbm` binding gets when it does not say. Matches the
/// default in `karakuri-store`'s `BindNoise`, which is the record this flag
/// stands in for.
const DEFAULT_OCTAVES: u32 = 4;

/// The `noise.kind` names, in the order the spec lists them. One list, so the
/// check and the message cannot drift apart.
const NOISE_KINDS: [&str; 4] = ["white", "value", "perlin", "fbm"];

fn parse_args_from(args: impl Iterator<Item = String>) -> Result<ParseOutcome, String> {
    let mut args_out = Args {
        overrides: Vec::new(),
        bindings: Vec::new(),
        bpm: DEFAULT_BPM,
        sets: Vec::new(),
        capacity: 262_144,
        render_to: None,
        seq_to: None,
        frames: 240,
        size: (1280, 720),
        watch: false,
        audio_in: None,
        latency_offset_ms: audio::DEFAULT_LATENCY_OFFSET_MS,
        budget_ms: DEFAULT_BUDGET_MS,
        look: Look {
            op: TonemapOp::Aces,
            exposure: 1.0,
            white_point: 1.0,
        },
    };
    let mut positional = Vec::new();
    let mut it = args;
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(ParseOutcome::Help),
            "--set" => {
                let value = value_for("--set", &mut it)?;
                match value.split_once(',') {
                    // A third part (a second comma) used to be accepted
                    // silently, taking `b,c` as one literal L4 filename — a
                    // typo like `--set a.kir,b.kir,c.kir` would compile-fail
                    // on a file that does not exist and blame the wrong
                    // thing. Rejected here instead, with the same message a
                    // missing comma gets.
                    Some((l1, l4)) if !l1.is_empty() && !l4.is_empty() && !l4.contains(',') => {
                        args_out.sets.push((PathBuf::from(l1), PathBuf::from(l4)))
                    }
                    _ => return Err(format!("`--set {value}` — expected `L1.kir,L4.kir`")),
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
                match value
                    .split_once('=')
                    .and_then(|(k, v)| v.parse().ok().map(|v| (k.to_string(), v)))
                {
                    Some(kv) => args_out.overrides.push(kv),
                    None => return Err(format!("`--param {value}` — expected `name=number`")),
                }
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
            "--audio-in" => {
                args_out.audio_in = Some(value_for("--audio-in", &mut it)?);
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
            "--budget-ms" => {
                args_out.budget_ms = number_for("--budget-ms", "a number of milliseconds", &mut it)?
            }
            "--frames" => args_out.frames = number_for("--frames", "a frame count", &mut it)?,
            "--capacity" => {
                args_out.capacity = number_for("--capacity", "an element count", &mut it)?
            }
            "--size" => {
                let value = value_for("--size", &mut it)?;
                match value.split_once('x') {
                    Some((w, h)) => match (w.parse(), h.parse()) {
                        (Ok(w), Ok(h)) => args_out.size = (w, h),
                        _ => return Err(format!("`--size {value}` — expected `WIDTHxHEIGHT`")),
                    },
                    None => return Err(format!("`--size {value}` — expected `WIDTHxHEIGHT`")),
                }
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
            .push((positional[0].clone(), positional[1].clone())),
        n => {
            return Err(format!(
                "{n} file argument(s) — a Set is an L1 and an L4, so give two, or use --set"
            ))
        }
    }
    if args_out.sets.is_empty() {
        args_out.sets.push((
            "examples/drift_shell.kir".into(),
            "examples/soft_points.kir".into(),
        ));
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
    if args_out.sets.len() > MAX_SLOTS {
        return Err(format!(
            "{} Sets, and the deck holds {MAX_SLOTS}",
            args_out.sets.len()
        ));
    }
    Ok(ParseOutcome::Run(Box::new(args_out)))
}

/// Each slot's seed, derived from its index alone so that two slots given the
/// same pair are not the same picture twice — and so that the derivation is a
/// pure function of the deck layout rather than of anything measured. Slot 0
/// is [`SEED`] unchanged, so a one-Set run renders exactly what it always did.
fn seed_for(slot: usize) -> u32 {
    SEED.wrapping_add((slot as u32).wrapping_mul(0x9E37_79B9))
}

type Pair = (karakuri_ir::typed::Checked, karakuri_ir::typed::Checked);

fn main() {
    let args = parse_args();

    eprintln!("compiling:");
    let mut procs: Vec<Pair> = Vec::new();
    for (slot, (l1, l4)) in args.sets.iter().enumerate() {
        eprintln!("  slot {slot}: {} + {}", l1.display(), l4.display());
        let load = |path: &PathBuf| match compile::load(path) {
            Ok(checked) => checked,
            Err(report) => {
                eprintln!("{report}");
                std::process::exit(1);
            }
        };
        procs.push((load(l1), load(l4)));
    }

    match args.render_to.clone().or(args.seq_to.clone()) {
        Some(path) => {
            let gpu = Gpu::headless().expect("no GPU");
            let (w, h) = args.size;
            // Fixed, never watching: an offscreen run is a function of its
            // inputs, and a save landing halfway through a sequence would make
            // it a function of the operator's editor as well.
            let mut deck = build_deck(&gpu, &procs, &args, false, false, w, h);
            eprintln!(
                "rendering the mix of {} Set{}, {w}x{h}, {} elements each, {} frames, \
                 {} at exposure {:.2} -> {}",
                deck.slot_count(),
                if deck.slot_count() == 1 { "" } else { "s" },
                args.capacity,
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
fn build_deck(
    gpu: &Gpu,
    procs: &[Pair],
    args: &Args,
    watch: bool,
    meters: bool,
    width: u32,
    height: u32,
) -> Deck {
    let swaps = procs
        .iter()
        .enumerate()
        .map(|(slot, (l1, l4))| {
            let set = build(
                gpu,
                l1,
                l4,
                args.capacity,
                &args.overrides,
                &args.bindings,
                seed_for(slot),
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
                    Box::new(watch::Watch::new(
                        slot,
                        args.sets[slot].0.clone(),
                        args.sets[slot].1.clone(),
                        args.capacity,
                        seed_for(slot),
                        args.overrides.clone(),
                        args.bindings.clone(),
                    )),
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
    let confidence = if binding.signal == NOISE_SIGNAL {
        signals.noise(&binding.noise.unwrap_or_default()).confidence
    } else {
        signals.sample(&binding.signal).confidence
    };
    let effect = if confidence >= 1.0 {
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

fn build(
    gpu: &Gpu,
    l1: &karakuri_ir::typed::Checked,
    l4: &karakuri_ir::typed::Checked,
    capacity: u32,
    overrides: &[(String, f32)],
    bindings: &[Binding],
    seed: u32,
) -> Set {
    match Set::build(&gpu.device, &gpu.queue, l1, l4, capacity, seed) {
        Ok(mut set) => {
            for (name, value) in overrides {
                match set.params.get_mut(name) {
                    Some(slot) => *slot = *value,
                    None => eprintln!("  no parameter named `{name}`, ignoring"),
                }
            }
            // After the overrides: a binding blends from the param's value, so
            // a `--param` on a bound param is the base of the blend rather
            // than a competitor for the write.
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
    procs: Option<Vec<Pair>>,
    live: Option<Live>,
}

struct Live {
    window: Arc<Window>,
    gpu: Gpu,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    present: Present,
    /// Every Set, whatever is being built to replace any of them, and the mix.
    /// Without `--watch` every slot is a `HotSwap::fixed` and there is no
    /// worker at all, so the frame loop below is the same code either way.
    deck: Deck,
    look: Look,
    /// The slot the gain keys act on. There is no on-screen UI, so this is
    /// printed on every change and marked in the status line.
    focus: usize,
    /// Fractional steps carried between frames, so a frame rate that does not
    /// divide the step rate still advances at the right average rate. The same
    /// accumulator shape as spawn quantisation, for the same reason.
    carry: f32,
    /// The audio input, the beat lock, and the operator's latency offset.
    /// `None` without `--audio-in`, and then nothing in the frame path below
    /// changes at all — which is the property the whole slice is about.
    audio: Option<audio::Audio>,
    /// The last measured frame interval, from [`Live::steps`].
    last_interval: f32,
    /// When the session started, so a tap has an origin to be measured from.
    /// The clock stays out here with the other one, for the same reason.
    started: Instant,
    last: Instant,
    status_at: Instant,
    frames_since_status: u32,
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
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&gpu.device, &config);

        let present = Present::new(&gpu.device, format, config.width, config.height);
        let mut deck = build_deck(
            &gpu,
            &procs,
            &self.args,
            self.args.watch,
            true,
            config.width,
            config.height,
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
            for (slot, (l1, l4)) in self.args.sets.iter().enumerate() {
                eprintln!(
                    "  watching slot {slot}: {} and {} — a save recompiles that slot in the \
                     background and swaps it when ready, budget {:.1} ms",
                    l1.display(),
                    l4.display(),
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

        eprint!("\n{BINDINGS}\n");

        let live = Live {
            window,
            gpu,
            surface,
            config,
            present,
            deck,
            look: self.args.look,
            focus: 0,
            carry: 0.0,
            audio,
            last_interval: DT,
            started: Instant::now(),
            last: Instant::now(),
            status_at: Instant::now(),
            frames_since_status: 0,
            status: String::with_capacity(256),
        };
        // Through a record at startup too, on the same terms as every later
        // change: `--tonemap` and `--exposure` are an operator's choices rather
        // than the engine's defaults, so a session that did not carry them
        // would replay under whatever look the next build happens to default
        // to. This is also the first thing that decodes one, so a `look` this
        // build cannot obey is reported before a frame is drawn.
        let mut live = live;
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
        if let Some(live) = &self.live {
            report_live_counts(&live.gpu, &live.deck);
        }
    }
}

impl Live {
    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.gpu.device, &self.config);
        // Both, always: the composite reads its sources with `textureLoad` and
        // an out-of-range load is defined to return zero, so a deck left at the
        // old size mixes black instead of failing. `Frame::render` asserts on
        // the pair for the same reason.
        self.present.resize(&self.gpu.device, width, height);
        self.deck.resize(&self.gpu.device, width, height);
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
                '\\' => self.set_gain(1.0),
                't' => self.cycle_tonemap(),
                '-' => self.set_exposure(self.look.exposure / EXPOSURE_STEP),
                '=' => self.set_exposure(self.look.exposure * EXPOSURE_STEP),
                '`' => self.set_exposure(1.0),
                'w' => self.toggle_priming(),
                'b' => self.tap(),
                ',' => self.shift_octave(0.5),
                '.' => self.shift_octave(2.0),
                'o' => self.nudge_latency_offset(-audio::LATENCY_OFFSET_STEP_MS),
                'p' => self.nudge_latency_offset(audio::LATENCY_OFFSET_STEP_MS),
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
        let slot = self.focus;
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
    fn toggle_priming(&mut self) {
        let slot = self.focus;
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
    fn govern(&mut self, why: &str) {
        let report = self.deck.govern();
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
        audio.tap(&mut signals, Instant::now(), started);
        self.deck.set_signals(signals);
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
            Some(_) => {
                self.deck.set_signals(signals);
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
        self.set_gain(self.deck.gain(self.focus) + delta);
    }

    fn set_gain(&mut self, gain: f32) {
        let slot = self.focus;
        self.record(mix::gain_record(slot, clamp_gain(gain)));
        eprintln!("slot {slot} gain {:.2}", self.deck.gain(slot));
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
    fn record(&mut self, record: karakuri_store::record::Record) {
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
            mix::Change::Residency { slot, level } => {
                self.deck.set_residency(slot, level);
                // A slot arriving or leaving changes what is committed, and the
                // deck's headroom with it: a slot that could not be admitted a
                // moment ago may fit now, and one that fitted may not. Requests
                // are untouched by the pass, so a park recovers on its own the
                // moment there is room.
                self.govern("residency");
            }
            mix::Change::Look(look) => {
                self.look = look;
                self.apply_look();
            }
        }
    }

    /// One `queue.write_buffer`. Not a pipeline rebuild, not a frame-boundary
    /// swap, and nothing for `HotSwap` to know about — which is what makes
    /// comparing operators on moving material possible at all.
    fn apply_look(&self) {
        self.present.set_tonemap(
            &self.gpu.queue,
            self.look.op,
            self.look.exposure,
            self.look.white_point,
        );
    }

    /// The one measurement in the program: elapsed real time becomes a step
    /// count, which is exactly what a `tick` record carries.
    fn steps(&mut self) -> u8 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_secs_f32();
        self.last = now;
        // Kept because the output lag a beat correction leads by starts with
        // the frame queue, which is a number of *frames* — and the frame rate
        // is the display's, not `dt`'s. See `audio::Audio::output_lag`.
        self.last_interval = elapsed;

        self.carry += elapsed / DT;
        let whole = self.carry.floor();
        self.carry -= whole;
        (whole as u32).min(u32::from(MAX_STEPS)) as u8
    }

    fn frame(&mut self) {
        let steps = self.steps();
        self.measure_audio(steps);

        let surface_frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.gpu.device, &self.config);
                return;
            }
            Err(_) => return,
        };
        let view = surface_frame.texture.create_view(&Default::default());

        // The guard owns the encoder, so everything recorded here is one
        // generation of Sets: builds are installed inside `begin_frame`, before
        // the encoder exists, and there is no way to reach a second generation
        // while this one is open. The present pass goes through
        // `Frame::encoder`, which is what that accessor is for.
        {
            let mut frame = self.deck.begin_frame(&self.gpu.device, &self.gpu.queue);
            frame.render(self.present.hdr_view(), self.present.size(), steps);
            self.present.draw(frame.encoder(), &view);
            frame.finish();
        }
        surface_frame.present();

        // A build landing replaces the Set in a slot, and with it the
        // measurement the deck is budgeting against — a swap and a rollback
        // both move what is committed. Collected here and governed after the
        // drain, because `Deck::events` borrows the deck for as long as it is
        // being read.
        let mut set_changed = false;
        for slot in 0..self.deck.slot_count() {
            for event in self.deck.events(slot) {
                set_changed |= matches!(event, Event::Swapped { .. } | Event::RolledBack { .. });
                eprintln!("slot {slot}: {event}");
            }
        }
        if set_changed {
            self.govern("build landed");
        }

        self.frames_since_status += 1;
        if self.status_at.elapsed() >= STATUS_INTERVAL {
            self.print_status();
        }
    }

    /// This frame's measurement, and what it does to the session.
    ///
    /// **Before the frame is rendered and never inside it.** The measured frame
    /// and the tempo correction are latched here, exactly where `steps` is
    /// measured, so that everything drawn this frame reads one set of values —
    /// two bindings sampling `energy` in one frame have to get one answer, or
    /// the record saying what this frame saw is a record of neither.
    ///
    /// The session's signals are taken by value, given this frame's
    /// measurement, and handed back. `Signals` is `Copy` and the copy carries
    /// the phase, so this is not the "restart the session clock" that
    /// `Deck::set_signals` warns about — it is the same clock with one frame's
    /// input attached.
    fn measure_audio(&mut self, steps: u8) {
        let Some(audio) = self.audio.as_mut() else {
            return;
        };
        let mut signals = *self.deck.signals();
        let (_audio_record, tempo) = audio.frame(
            &mut signals,
            self.last_interval,
            f32::from(steps) * DT,
        );
        self.deck.set_signals(signals);

        // A correction is worth saying out loud when it is a decision rather
        // than a trim: acquiring, re-acquiring, and a tap all move the grid at
        // once, and an operator who cannot see that happen cannot tell a lock
        // from a coincidence.
        if let (Some(karakuri_store::record::Record::Tempo { bpm, .. }), Some(reason)) =
            (tempo, audio.reason())
        {
            if !matches!(reason, karakuri_audio::Reason::Trim) {
                eprintln!("beat: {reason:?} at {bpm:.1} bpm");
            }
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
            // The level, which is why the meter exists: two Sets are matched
            // on `m` and `p` warns which one will dominate the mix wherever it
            // lands regardless of its fader. `None` for an off-air slot is the
            // meter saying it has nothing current rather than showing the last
            // thing the slot drew — see `Deck::level`.
            match self.deck.level(slot) {
                Some(level) => {
                    let _ = write!(self.status, "m{:.3} p{:.1}  ", level.mean, level.peak);
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

    #[test]
    fn no_arguments_is_the_default_pair() {
        let args = parse(&[]).expect("should parse");
        assert_eq!(
            args.sets,
            vec![(
                PathBuf::from("examples/drift_shell.kir"),
                PathBuf::from("examples/soft_points.kir"),
            )]
        );
    }

    #[test]
    fn a_bare_positional_pair_still_works() {
        let args = parse(&["a.kir", "b.kir"]).expect("should parse");
        assert_eq!(
            args.sets,
            vec![(PathBuf::from("a.kir"), PathBuf::from("b.kir"))]
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
                (PathBuf::from("a.kir"), PathBuf::from("b.kir")),
                (PathBuf::from("c.kir"), PathBuf::from("d.kir")),
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

    #[test]
    fn set_with_three_paths_fails_rather_than_taking_a_comma_in_the_filename() {
        // `b.kir,c.kir` used to be silently accepted as one literal L4 path.
        let err = parse(&["--set", "a.kir,b.kir,c.kir"]).unwrap_err();
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
                octaves: DEFAULT_OCTAVES
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
            ("layer=L2,key=hue,signal=beat,range=0..1", "expected L1 or L4"),
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
            (vec!["--param", "turbulence"], "--param"),
        ] {
            let err = parse(&args).unwrap_or_else(|| panic!("{args:?} was accepted"));
            assert!(err.contains(expect), "{args:?} -> {err}");
        }
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
