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

mod compile;
mod render;
mod watch;

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use karakuri_engine::deck::MAX_SLOTS;
use karakuri_engine::{Deck, Gpu, HotSwap, Present, Residency, Set, TonemapOp, DEFAULT_BUDGET_MS};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// The fixed simulation step. Not the frame delta.
const DT: f32 = 1.0 / 60.0;

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
  [ ]        focused slot gain down / up
  \\          focused slot gain back to 1.0
  t          cycle the tone map operator: clamp, Reinhard, ACES, AgX
  - =        output exposure down / up
  `          exposure back to 1.0
  s          print the status line now
  h          print these bindings
  esc        quit
";

/// The output look: everything the tone mapper is told, in one value, so the
/// window and an offscreen render can be given the same thing and agree.
#[derive(Clone, Copy)]
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

fn op_name(op: TonemapOp) -> &'static str {
    match op {
        TonemapOp::Clamp => "clamp",
        TonemapOp::Reinhard => "Reinhard",
        TonemapOp::Aces => "ACES",
        TonemapOp::AgX => "AgX",
    }
}

#[cfg_attr(test, derive(Debug))]
struct Args {
    /// `--param name=value`, applied after each Set is built, to every Set. A
    /// parameter change is a uniform write, not a structural change, which is
    /// why it needs no fork and no recompilation.
    overrides: Vec<(String, f32)>,
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

fn parse_args_from(args: impl Iterator<Item = String>) -> Result<ParseOutcome, String> {
    let mut args_out = Args {
        overrides: Vec::new(),
        sets: Vec::new(),
        capacity: 262_144,
        render_to: None,
        seq_to: None,
        frames: 240,
        size: (1280, 720),
        watch: false,
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
            "--tonemap" => {
                let value = value_for("--tonemap", &mut it)?;
                args_out.look.op = match value.as_str() {
                    "clamp" => TonemapOp::Clamp,
                    "reinhard" => TonemapOp::Reinhard,
                    "aces" => TonemapOp::Aces,
                    "agx" => TonemapOp::AgX,
                    _ => {
                        return Err(format!(
                            "`--tonemap {value}` — expected clamp, reinhard, aces or agx"
                        ))
                    }
                };
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
            let set = build(gpu, l1, l4, args.capacity, &args.overrides, seed_for(slot));
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
                    )),
                )
            } else {
                HotSwap::fixed(set)
            }
        })
        .collect();
    let mut deck = Deck::new(&gpu.device, swaps, width, height);
    if meters {
        deck.enable_meters(&gpu.device);
    }
    deck
}

fn build(
    gpu: &Gpu,
    l1: &karakuri_ir::typed::Checked,
    l4: &karakuri_ir::typed::Checked,
    capacity: u32,
    overrides: &[(String, f32)],
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
        let deck = build_deck(
            &gpu,
            &procs,
            &self.args,
            self.args.watch,
            true,
            config.width,
            config.height,
        );

        eprintln!(
            "running: {} slot{} of {} elements on {}",
            deck.slot_count(),
            if deck.slot_count() == 1 { "" } else { "s" },
            self.args.capacity,
            gpu.adapter.get_info().name
        );
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
            last: Instant::now(),
            status_at: Instant::now(),
            frames_since_status: 0,
            status: String::with_capacity(256),
        };
        // The uniform the shader already reads: switching the operator is this
        // call and nothing else, at startup and on every `t` afterwards.
        live.apply_look();
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
            residency_name(self.deck.residency(slot)),
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
                self.deck.set_residency(slot, Residency::Allocated);
                eprintln!("slot {slot} off air — allocated, holding t {t:.2}s");
            }
            Residency::Allocated => {
                self.deck.set_residency(slot, Residency::Live);
                eprintln!("slot {slot} on air — resuming at t {t:.2}s");
            }
        }
    }

    fn nudge_gain(&mut self, delta: f32) {
        self.set_gain(self.deck.gain(self.focus) + delta);
    }

    fn set_gain(&mut self, gain: f32) {
        let gain = clamp_gain(gain);
        self.deck.set_gain(self.focus, gain);
        eprintln!("slot {} gain {gain:.2}", self.focus);
    }

    fn cycle_tonemap(&mut self) {
        self.look.op = next_tonemap(self.look.op);
        self.apply_look();
        eprintln!(
            "tonemap {} (exposure {:.2})",
            self.look.name(),
            self.look.exposure
        );
    }

    fn set_exposure(&mut self, exposure: f32) {
        self.look.exposure = clamp_exposure(exposure);
        self.apply_look();
        eprintln!("exposure {:.3} ({})", self.look.exposure, self.look.name());
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

        self.carry += elapsed / DT;
        let whole = self.carry.floor();
        self.carry -= whole;
        (whole as u32).min(u32::from(MAX_STEPS)) as u8
    }

    fn frame(&mut self) {
        let steps = self.steps();

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

        for slot in 0..self.deck.slot_count() {
            for event in self.deck.events(slot) {
                eprintln!("slot {slot}: {event}");
            }
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
                match self.deck.residency(slot) {
                    Residency::Live => "LIVE",
                    Residency::Allocated => "off ",
                },
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
        }
        eprintln!(
            "{}| {} exp {:.2} | {fps:.1} fps",
            self.status,
            self.look.name(),
            self.look.exposure
        );
    }
}

fn residency_name(residency: Residency) -> &'static str {
    match residency {
        Residency::Live => "live",
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

    #[test]
    fn tonemap_cycles_through_all_four_and_back_to_the_start() {
        let start = TonemapOp::Clamp;
        let mut op = start;
        let mut seen = vec![op];
        for _ in 0..3 {
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
