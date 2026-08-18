//! Hot swap: building a Set off the render thread, installing it on a frame
//! boundary, and taking it back out again if it does not hold up.
//!
//! This is the third clause of the V1 assumption — "and can we hot-swap it
//! without dropping a frame?" — and it is the one place where all four render
//! thread invariants have to hold at once:
//!
//! - **Never allocate on the render thread. Never compile shaders on it.**
//! - Pipelines are double-buffered; swaps happen only on frame boundaries.
//! - If a new pipeline exceeds the frame budget, roll back automatically.
//!
//! ## Compiling somewhere else
//!
//! `wgpu::Device` and `wgpu::Queue` are `Send + Sync` and cheap to clone, and
//! `Set` is `Send` — nothing in it is a raw handle, a `Rc`, or a cell (see the
//! static assertion at the bottom of this module, which is there so that a
//! future field which *is* one of those fails to compile here rather than
//! silently forcing this whole design back onto the render thread). So a
//! worker thread can do the entire expensive half of the job: parse-checked IR
//! in, WGSL out, shader modules, pipelines, buffers, bind groups, a whole
//! `Set`, handed over on a channel already finished.
//!
//! The render thread's side of that channel is `try_recv`, never `recv`, and
//! there is no `device.poll(PollType::Wait)` anywhere on the frame path. A
//! frame that finds nothing waiting does nothing about it and renders.
//!
//! ## Where a swap lands, and what actually guarantees it
//!
//! [`HotSwap::begin_frame`] is the only place `live` is replaced, and it
//! returns `&mut Set` borrowed from `&mut self`. While a caller holds that
//! borrow the borrow checker will not let it call `begin_frame` — or anything
//! else on the `HotSwap` — again, so no *single* `&mut Set` can change
//! identity underneath a frame.
//!
//! **That is weaker than "a swap cannot land mid-frame", and the difference
//! matters.** The command encoder is the caller's, and it borrows nothing from
//! the `HotSwap`. A caller is free to open one encoder, drop the borrow, call
//! `begin_frame` again, and record a second Set's passes into the same
//! encoder; that compiles, and if a build arrived in between the encoder ends
//! up holding half of one Set's frame and half of another's. The property this
//! module wants is "one `begin_frame` per encoder", and nothing in *these*
//! types says so — it is a convention `karakuri-cli`'s frame loop and the
//! harness in `tests/hot_swap.rs` both happen to keep.
//!
//! Making it structural means giving `begin_frame` the encoder, and
//! [`crate::deck`] now does exactly that: `Deck::begin_frame` returns a guard
//! owning both the `&mut Deck` and the `CommandEncoder`, so a second frame
//! cannot be opened while one is and there is no way to record two generations
//! of Sets into one encoder. That is where a deck of one to four Sets is
//! composited, and it is the shape a caller with more than one Set has to use.
//!
//! A `HotSwap` driven on its own — which is what `karakuri-cli` and
//! `tests/hot_swap.rs` still do — keeps the weaker property, and the honest
//! claim for it is unchanged: **the live Set is only ever replaced at the top
//! of `begin_frame`**, and such a caller must call it exactly once per frame.
//!
//! ## What a swap does not do
//!
//! It transfers no state. A new procedure means new buffers, so the incoming
//! Set starts cold: `t` at zero, element buffers at their initial contents,
//! nothing primed. That is correct for V1. Warming a Set out of sight before
//! showing it is `docs/roadmap.md`'s M2 (Priming) and needs the deck and the
//! residency model to exist first; half of it built here would be a second,
//! worse answer that M2 would then have to remove.
//!
//! The outgoing Set, by contrast, is not stepped while it waits to see whether
//! it is needed again — `t` is simulation time and does not advance for a Set
//! nothing is calling `prepare` on. A rollback therefore resumes it exactly
//! where it was parked, which is the same property M2 names *Allocated*
//! residency.
//!
//! ## The watchdog measures a host clock, and says so
//!
//! What is measured is the interval between successive `begin_frame` calls:
//! how often frames are actually coming out. Not the GPU cost of the render
//! pass — `crates/karakuri-engine/src/probe.rs` explains at length why GPU
//! timestamps are advertised, enabled, and unreliable on this crate's
//! development machine, and a watchdog steering on a number that reads 0.0 ms
//! for a genuinely heavy workload is worse than no watchdog at all. A host
//! clock cannot separate shader cost from vsync, from the compositor, or from
//! whatever else the machine is doing. It can tell that frames stopped
//! arriving on time, which is the thing "exceeds the frame budget" means on
//! stage, and it is honest about being that and nothing more.
//!
//! Two consequences of it being a frame *interval*: under vsync it quantises
//! to multiples of the refresh period, so the budget has to sit between one
//! period and two ([`DEFAULT_BUDGET_MS`]); and the verdict is a **median**
//! over a window rather than a worst case, because on a host clock a single
//! sample carries whatever else the OS scheduler was doing. `Probe` makes the
//! same choice for the same reason. One hitch is not a reason to throw away
//! generated material; thirty frames that all miss is.
//!
//! ## The window, and why it does not start at frame one
//!
//! A cold Set's first frames pay for things no steady-state frame pays for:
//! the driver's first use of each freshly created pipeline, first touch of
//! freshly allocated buffers, and — the big one — the whole-capacity element
//! and alive buffer upload `Set::build` leaves staged on the queue, which at
//! 262144 elements is megabytes. A budget check that fired on frame one would
//! reject every candidate that ever existed, forever, and the symptom would be
//! a hot-swap feature that appears to work and never keeps anything.
//!
//! So [`WARMUP_FRAMES`] are discarded before [`JUDGE_FRAMES`] are measured.
//! The worker also flushes and waits for that upload on its own thread before
//! handing the Set over, which removes most of the cost from the window rather
//! than merely hiding it inside the warmup.
//!
//! ## The worker also measures what it built
//!
//! The watchdog above is a *frame interval*, and a frame interval cannot be
//! divided among the slots that produced it — every Live slot in a deck is
//! judged against the whole deck's, so a budget that fits one Set rolls back
//! every candidate in a deck of four. That defect is fatal for a governor,
//! which has to add up what several Sets cost and decide, so the governor does
//! not use it. It uses a **per-Set measurement taken here**, on the worker
//! thread, as part of building the Set.
//!
//! This is the right place for three reasons and they are all already written
//! down in this repository:
//!
//! - The worker already does a submit-and-wait before handing a Set over (see
//!   "The window" above), so there is somewhere to measure that is not the
//!   render thread. [`crate::probe`]'s whole method is submit, wait, repeat.
//! - `crate::probe` already knows that GPU timestamps are advertised, enabled
//!   and unreliable on this machine, calibrates against a known-heavy workload
//!   rather than trusting the feature flag, and labels every number with how it
//!   was obtained. A governor steering on an unlabelled number is the failure
//!   mode `docs/roadmap.md` asks to decide about before building this.
//! - A Set is the unit the governor budgets in, and it is the unit that gets
//!   built. Measuring it where it is built means the measurement travels with
//!   it and cannot get attached to the wrong one.
//!
//! **What the measurement is of**, exactly: one frame of this Set at
//! [`PROBE_STEPS`] simulation steps, at its real capacity and with its real
//! parameters and bindings already applied, rendered into an offscreen target
//! of [`PROBE_RESOLUTION`] — its L1 compute passes and its L4 draw, the
//! commands `VideoSource::render` records and nothing else.
//!
//! **What it cannot see**, and every one of these matters to whatever reads it:
//!
//! - **Resolution.** It is taken at a fixed reference size, not at the deck's.
//!   L4 cost is fill-rate bound, so a slot on a 4K output costs several times
//!   this. The number is comparable *between slots* — which is what a budget
//!   needs — and is not a prediction of this machine's frame time.
//! - **The composite, the meters, the present pass, and `prepare`.** None of
//!   them is inside `render`. The deck's own per-frame cost is not in here.
//! - **The future.** It is one measurement of a cold Set at one instant. A
//!   procedure whose population grows, whose points get bigger, or whose
//!   overdraw rises as it spreads costs more later, and nothing re-measures.
//! - **Anything host-side**, on the GPU path; and on the fallback path it
//!   includes submission and synchronization overhead and reads biased high.
//!   [`Measurement::method`] is which, and it is not decoration.
//!
//! Measuring means *stepping* the Set, which a Set about to be swapped in must
//! not arrive having done — it is documented as arriving cold. So the probe run
//! is followed by [`Set::rewind`], which puts the buffers, `t`, parity and the
//! spawn accumulator back to what `Set::build` left. That is another
//! whole-capacity upload on the worker thread, next to the one that was already
//! there.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use karakuri_ir::typed::Checked;

use crate::binding::{Binding, Signals};
use crate::probe::{Measurement, Probe};
use crate::set::{Set, SetError};

/// Frames discarded after a swap, before the watchdog starts measuring. See
/// "The window" in the module doc: the alternative is rolling everything back.
const WARMUP_FRAMES: u32 = 8;

/// Frames the watchdog measures before deciding. Half a second at 60 Hz —
/// long enough that the verdict is not one sample's opinion on a host clock,
/// short enough that a Set which cannot hold the budget is not on screen for
/// long. Both halves of that matter: a shorter window makes the watchdog fire
/// on noise, a longer one is half a phrase of a track spent looking wrong.
const JUDGE_FRAMES: usize = 30;

/// The default frame budget, in milliseconds: one 60 Hz frame plus slack.
///
/// Deliberately not 16.7. What the watchdog measures is the interval between
/// frames on a host clock, and under vsync a *healthy* frame sits right at the
/// refresh period — a budget set there would roll back every candidate on
/// rounding. A frame that misses vsync lands at the next period, because there
/// is nothing in between. 60 Hz is the rate the simulation itself runs at
/// (`dt` is 1/60), so "below 60 fps" is the condition worth testing and 20 ms
/// is the threshold that tests it.
///
/// **This does not generalise to a faster display, and the development machine
/// is one.** On a 120 Hz panel a healthy frame is 8.3 ms and a frame rate cut
/// in half reads as 16.7 ms — under this budget, so the watchdog would keep a
/// candidate that halved the frame rate. `karakuri-cli`'s `--budget-ms` is the
/// operator's answer to that. The real answer is a budget derived from the
/// display rather than from a constant, which belongs with `docs/roadmap.md`'s
/// M2 budget governor: that is where per-Set measurement and the decision
/// about whether GPU timestamps can be trusted on the performing machine both
/// live, and picking a second, worse answer here would be something M2 has to
/// remove.
pub const DEFAULT_BUDGET_MS: f32 = 20.0;

/// The offscreen size every per-Set measurement is taken at.
///
/// Fixed rather than the deck's, and that is a trade rather than an oversight.
/// The deck's size is render-thread state; the worker would have to be told it,
/// and a measurement taken at whatever the window happened to be would not be
/// comparable with one taken a drag-resize earlier. A governor adds
/// measurements together and compares them, so **comparable matters more than
/// absolute** — and an absolute number would be a lie the moment the window
/// moved anyway. 1280x720 because it is the size every other figure in
/// `README.md` was taken at.
pub const PROBE_RESOLUTION: (u32, u32) = (1280, 720);

/// Simulation steps per measured frame. One, because that is what a `tick`
/// carries in normal play; a Set that falls behind and substeps costs a
/// multiple of this, which is the caller's arithmetic rather than the probe's.
pub const PROBE_STEPS: u8 = 1;

/// How long a [`Source`] with nothing to report should block before returning.
/// The worker does nothing else between polls, so a source that returns
/// immediately turns it into a spin.
pub const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Everything the worker needs to build one Set. Owned, not borrowed: the
/// worker is on another thread and cannot hold anything belonging to the
/// render loop.
pub struct Request {
    /// The caller's own name for this build, echoed back on every event about
    /// it.
    ///
    /// **The engine never interprets it and never invents one.** What it is for
    /// is the thing a caller cannot otherwise do: say *which* build landed.
    /// `label` is for a human and repeats on every rebuild of the same pair,
    /// and the queue collapses superseded builds, so counting does not work
    /// either — a caller recording what a session actually played needs to
    /// match an outcome to the source that produced it, and this is the only
    /// thread between them.
    pub id: u64,
    /// **The geometry sources, each with the capacity it runs at.** A list
    /// because a Set holds a list — see `crate::set::Source` — and the capacity
    /// travels beside each one because each declares its own range, so one
    /// number cannot serve two.
    pub l1s: Vec<(Checked, u32)>,
    /// The deformations, in chain order — each reads what the one before wrote.
    /// Empty for a Set that draws its geometry as the L1 made it.
    pub l2s: Vec<Checked>,
    /// The camera, or `None` to leave it the built-in orbit. At most one: a Set
    /// is a grouping around one viewpoint, and two viewpoints composited is a
    /// graph rather than a Set.
    pub l3: Option<Checked>,
    /// The field, or `None` for a Set that evaluates none. At most one, on the
    /// same terms as the camera — see [`karakuri_ir::Kind::Field`].
    pub field: Option<Checked>,
    /// The renderers, in draw order — see [`Set::build_many`]. A rebuild names
    /// every one of them rather than the one that changed, for the reason the
    /// params below are restated: a request that depended on what happens to be
    /// live is not reproducible from a record stream.
    pub l4s: Vec<Checked>,
    /// Whether the renderers overdraw or composite — see
    /// [`crate::set::Layering`]. Restated on every rebuild for the reason the
    /// params below are: a request that depended on what happens to be live is
    /// not reproducible from a record stream.
    pub layering: crate::set::Layering,
    pub seed_salt: u32,
    /// Applied to the new Set once it is built. Parameter values are the one
    /// piece of Set state that is not structural, so they are the one thing
    /// worth carrying across a swap — and they are carried by being *restated*
    /// here rather than read out of the outgoing Set, because a request that
    /// depends on what happens to be live is not reproducible from a record
    /// stream.
    pub params: Vec<crate::binding::ParamWrite>,
    /// **The interface, restated on every rebuild** for the reason the params
    /// and the bindings are: a request that depended on what happens to be live
    /// is not reproducible from a record stream.
    ///
    /// Left out, this was worse than a lost surface. The bindings *are*
    /// restated, so a `control:` binding survived a swap and the control it
    /// named did not — and a binding whose source is gone leaves its param
    /// where it was, silently, for the rest of the run.
    pub published: Vec<crate::set::Published>,
    /// Attached to the new Set once its params are set, and **restated** here
    /// for the same reason they are: a rebuild that read its bindings out of
    /// whatever happened to be live would not be reproducible from a record
    /// stream. A binding is Set state, not Set structure — the new Set is a
    /// new value either way, and a `bind` record travels with it.
    pub bindings: Vec<Binding>,
    /// What a swap or rollback message calls this.
    pub label: String,
}

/// Where the worker gets its work.
///
/// Implemented by the caller rather than here, because "what changed" is not
/// the engine's business: a file watcher, a record stream, and M6's generation
/// queue are the same shape from this side. A source that decides there is
/// nothing to build — including because a `.kir` file failed to compile and it
/// printed diagnostics instead — simply returns `None`, and nothing happens.
/// That is how "a failed compile changes nothing" is enforced: a failure never
/// produces a [`Request`] in the first place, so it cannot reach the render
/// thread to be rejected there.
///
/// Called only on the worker thread. `poll` is expected to block for roughly
/// [`POLL_INTERVAL`] when it has nothing to report.
pub trait Source: Send {
    fn poll(&mut self) -> Option<Request>;
}

/// A channel is the simplest source there is: whoever holds the `Sender`
/// decides when a rebuild happens. Tests use this, and so would an agent
/// queueing generated material.
impl Source for Receiver<Request> {
    fn poll(&mut self) -> Option<Request> {
        match self.recv_timeout(POLL_INTERVAL) {
            Ok(request) => Some(request),
            Err(RecvTimeoutError::Timeout) => None,
            // `Disconnected` returns immediately rather than after the
            // timeout, so sleep by hand — otherwise a dropped `Sender` turns
            // the worker into a spin until the `HotSwap` is dropped.
            Err(RecvTimeoutError::Disconnected) => {
                std::thread::sleep(POLL_INTERVAL);
                None
            }
        }
    }
}

/// What happened, in the order it happened. Drained by the caller and printed;
/// the engine does not print, so that a test can assert on the same values a
/// user reads.
#[derive(Debug)]
pub enum Event {
    /// A build finished and is now the live Set. It is on trial until the
    /// watchdog reports on it.
    Swapped { id: u64, label: Arc<str> },
    /// A build failed. **Nothing changed**: the running Set is still running,
    /// with its `t` and its live count untouched.
    Rejected {
        id: u64,
        label: Arc<str>,
        error: SetError,
    },
    /// The watchdog's verdict, in favour. The previous Set is released.
    Accepted {
        id: u64,
        label: Arc<str>,
        median_ms: f32,
        /// Reported alongside, because "held the budget" is not a useful
        /// thing to read without the number it held against — the default is
        /// derived from 60 Hz and a display running faster than that can halve
        /// its frame rate and still come in under it.
        budget_ms: f32,
    },
    /// The watchdog's verdict, against. The previous Set is live again, at the
    /// `t` it was parked at, and the candidate is released.
    RolledBack {
        id: u64,
        label: Arc<str>,
        median_ms: f32,
        budget_ms: f32,
    },
    /// The build worker is gone — it can only leave by panicking — so nothing
    /// will be built again for the rest of the run. Emitted once. The live Set
    /// keeps running; what stops is the ability to replace it.
    WorkerLost,
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Swapped { label, .. } => {
                write!(f, "swapped in `{label}` — cold, t back to zero")
            }
            Event::Rejected { label, error, .. } => {
                write!(f, "`{label}` was refused, nothing changed:\n{error}")
            }
            Event::Accepted {
                label,
                id: _,
                median_ms,
                budget_ms,
            } => write!(
                f,
                "`{label}` held the budget: {median_ms:.2} ms median frame interval \
                 against {budget_ms:.2} ms (host clock)"
            ),
            Event::RolledBack {
                label,
                id: _,
                median_ms,
                budget_ms,
            } => write!(
                f,
                "rolled back `{label}`: {median_ms:.2} ms median frame interval over \
                 {JUDGE_FRAMES} frames exceeds the {budget_ms:.2} ms budget \
                 (host clock — see README's Working style)"
            ),
            Event::WorkerLost => write!(
                f,
                "the build worker is gone; nothing more will be built this run \
                 (the live Set is unaffected)"
            ),
        }
    }
}

/// A finished build on its way back to the render thread.
struct Built {
    id: u64,
    label: Arc<str>,
    result: Result<Set, SetError>,
    /// What the worker measured this Set at, if it got that far. Travels with
    /// the Set rather than being looked up later, so it cannot be attached to
    /// the wrong one — see "The worker also measures what it built".
    cost: Option<Measurement>,
}

/// The candidate currently being watched.
struct Trial {
    id: u64,
    label: Arc<str>,
    /// Frame intervals attributed to this Set so far, warmup included.
    seen: u32,
}

/// A live Set, a worker building the next one, and a watchdog over the swap.
///
/// Construct with [`HotSwap::new`] to get the worker, or [`HotSwap::fixed`]
/// for a run that will never swap — the frame loop is then identical either
/// way, which is the point of routing both through here.
pub struct HotSwap {
    live: Set,
    /// What the worker measured [`HotSwap::live`] at, if anything did. `None`
    /// for a Set no worker built — [`HotSwap::fixed`]'s, and the one `new` was
    /// constructed with — until [`HotSwap::set_measured_cost`] supplies one.
    cost: Option<Measurement>,
    /// The Set that was live before the current one, held so that a rollback
    /// is a move rather than a rebuild. Not stepped while parked; see "What a
    /// swap does not do" in the module doc.
    previous: Option<Set>,
    /// Its measurement, parked with it. A rollback restores both, or the
    /// governor would go on budgeting for a Set that is no longer there.
    previous_cost: Option<Measurement>,
    trial: Option<Trial>,
    /// Frame intervals in the current judging window. Capacity is fixed at
    /// [`JUDGE_FRAMES`] here so that `push` on the render thread never grows
    /// it.
    samples: Vec<f32>,
    budget_ms: f32,
    viewport: (u32, u32),
    last_frame: Option<Instant>,
    frames: u64,
    /// Sized for the most that can accumulate between two drains, and every
    /// `Event` owns its strings already, so pushing one allocates nothing. A
    /// caller that stops draining eventually makes this grow — which is a
    /// caller bug rather than a case to handle here, because the alternative
    /// is silently discarding a rollback nobody was told about.
    events: Vec<Event>,

    built: Receiver<Built>,
    /// Sets the render thread is done with, waiting for the worker to drop
    /// them. Dropping a Set releases its buffers, bind groups and pipelines,
    /// and a deallocation on the render thread is the same invariant as an
    /// allocation on it.
    graveyard: Arc<Mutex<Vec<Set>>>,
    /// Retired but not yet handed over, because the worker held the graveyard
    /// lock this frame. Retried next frame rather than dropped here.
    retired: Vec<Set>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    /// The worker was expected and its channel is closed, and that has been
    /// reported once. A worker only closes it by panicking — `run_worker`
    /// otherwise runs until [`HotSwap::drop`] — and the symptom without this
    /// is a `--watch` session that silently stops responding to saves.
    worker_lost: bool,
}

impl HotSwap {
    /// A Set with a worker behind it. `source` is polled on the worker thread;
    /// anything it produces is built there and offered to the render thread on
    /// the next frame boundary.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        live: Set,
        budget_ms: f32,
        source: Box<dyn Source>,
    ) -> HotSwap {
        let (built_tx, built_rx) = mpsc::channel();
        let graveyard = Arc::new(Mutex::new(Vec::with_capacity(GRAVEYARD_CAPACITY)));
        let stop = Arc::new(AtomicBool::new(false));

        let worker = {
            // Both are `Arc`s inside, so this is a refcount bump rather than a
            // second device — the worker builds against the same device the
            // render thread renders with, which is the whole reason the
            // finished pipelines are usable when they arrive.
            let device = device.clone();
            let queue = queue.clone();
            let graveyard = Arc::clone(&graveyard);
            let stop = Arc::clone(&stop);
            std::thread::Builder::new()
                .name("karakuri-build".into())
                .spawn(move || run_worker(device, queue, source, built_tx, graveyard, stop))
                .expect("spawn build worker")
        };

        HotSwap {
            live,
            cost: None,
            previous: None,
            previous_cost: None,
            trial: None,
            samples: Vec::with_capacity(JUDGE_FRAMES),
            budget_ms,
            viewport: (1, 1),
            last_frame: None,
            frames: 0,
            events: Vec::with_capacity(EVENT_CAPACITY),
            built: built_rx,
            graveyard,
            retired: Vec::with_capacity(GRAVEYARD_CAPACITY),
            stop,
            worker: Some(worker),
            worker_lost: false,
        }
    }

    /// Put `set` in, now, with no worker and no trial.
    ///
    /// **For a replay, and only a replay.** A live run's swaps arrive from a
    /// worker and are judged for thirty frames against the budget; a replay is
    /// reading what a live run already decided, out of a `procedure` record, so
    /// there is nothing left to judge. Judging again would be worse than
    /// pointless — an offscreen render has no frame budget to fail, and a
    /// rollback the live run did not have would put the replay on a procedure
    /// the performance never showed.
    ///
    /// The outgoing Set is retired rather than parked: there is no rollback to
    /// park it for. Its measured cost goes with it, so the governor treats the
    /// incoming one as unmeasured — which is what it is.
    pub fn install(&mut self, device: &wgpu::Device, mut set: Set) {
        set.resize(device, self.viewport.0, self.viewport.1);
        let outgoing = std::mem::replace(&mut self.live, set);
        self.retire(outgoing);
        self.cost = None;
        self.previous = None;
        self.previous_cost = None;
        self.trial = None;
        self.samples.clear();
    }

    /// One Set and no worker, for `--render`, `--seq`, and a window run
    /// without `--watch`. Nothing will ever be swapped in, so nothing is ever
    /// measured against a budget either — but the frame loop is the same one.
    pub fn fixed(live: Set) -> HotSwap {
        // A receiver whose sender is already gone: `try_recv` says
        // `Disconnected` forever, which `install_if_ready` treats exactly like
        // "nothing waiting".
        let (_, built_rx) = mpsc::channel();
        HotSwap {
            live,
            cost: None,
            previous: None,
            previous_cost: None,
            trial: None,
            samples: Vec::new(),
            budget_ms: f32::INFINITY,
            viewport: (1, 1),
            last_frame: None,
            frames: 0,
            events: Vec::with_capacity(EVENT_CAPACITY),
            built: built_rx,
            graveyard: Arc::new(Mutex::new(Vec::new())),
            retired: Vec::new(),
            stop: Arc::new(AtomicBool::new(false)),
            worker: None,
            worker_lost: false,
        }
    }

    /// The top of a frame, and the only place the live Set is ever replaced.
    ///
    /// Everything that can change which Set is live happens here, before the
    /// borrow is handed out: the previous frame's interval is fed to the
    /// watchdog (which may roll back), and then a finished build is installed
    /// if one has arrived. The caller records its whole frame through the
    /// returned reference, and cannot touch this `HotSwap` again until it
    /// drops it.
    ///
    /// **Call this exactly once per frame.** Calling it a second time inside
    /// one command encoder compiles, and records two Sets' passes into that
    /// encoder; see "Where a swap lands, and what actually guarantees it" in
    /// the module doc for why the borrow alone does not rule that out.
    /// [`crate::deck::Deck::begin_frame`] is the version that does rule it
    /// out, by owning the encoder — a `HotSwap` driven directly still relies
    /// on the caller.
    ///
    /// Allocates nothing, compiles nothing, and never blocks: the channel is
    /// polled with `try_recv` and the graveyard with `try_lock`.
    pub fn begin_frame(&mut self, device: &wgpu::Device) -> &mut Set {
        self.frame_boundary(device, true);
        &mut self.live
    }

    /// The same frame boundary for a slot that is **not on air**: a finished
    /// build is still installed and retired Sets are still handed back to the
    /// worker, but the frame interval is not fed to the watchdog.
    ///
    /// A candidate in an off-air slot renders nothing, so the frame interval
    /// the caller is producing is entirely other slots' cost. Judging against
    /// it accepts a candidate on a budget it never spent — and, with a tight
    /// budget and busy neighbours, rolls one back for cost it never caused.
    /// So the trial is *frozen* instead: `seen` does not advance, no sample is
    /// taken, and the verdict waits until the slot is Live and has actually
    /// paid for [`JUDGE_FRAMES`] frames of its own.
    ///
    /// That is the same reasoning that parks an outgoing Set's `t` across a
    /// window — a Set that is not running is not measured either — and it is
    /// as far as an interval-based watchdog can get. What it still cannot do
    /// is separate one Live slot's cost from its neighbours': every Live slot
    /// in a deck is judged against the whole deck's frame interval, so a
    /// budget that fits one Set rolls back every candidate in a deck of four.
    /// Fixing *that* needs a per-Set measurement, which is M2's budget
    /// governor.
    pub(crate) fn begin_frame_parked(&mut self, device: &wgpu::Device) {
        self.frame_boundary(device, false);
    }

    fn frame_boundary(&mut self, device: &wgpu::Device, on_air: bool) {
        let now = Instant::now();
        let last = self.last_frame.replace(now);
        if on_air {
            // The interval that just ended is the *previous* frame's duration,
            // so the watchdog is always one frame behind. It has to be: a
            // frame's cost is not known until the next one starts.
            if let Some(last) = last {
                self.record(now.duration_since(last).as_secs_f32() * 1_000.0);
            }
        } else {
            // Not a sample, and not the left-hand end of one either: the first
            // frame back on air would otherwise be measured as however long
            // the slot spent off it.
            self.last_frame = None;
        }
        self.frames += 1;
        self.hand_over_retired();
        self.install_if_ready(device);
    }

    /// The live Set, outside a frame. Read-only, so it cannot be rendered
    /// through — recording a frame goes through [`HotSwap::begin_frame`], and
    /// that is deliberate.
    pub fn set(&self) -> &Set {
        &self.live
    }

    /// The live Set, mutably, with **none** of the frame-boundary work
    /// [`HotSwap::begin_frame`] does: no watchdog sample, no install, no
    /// handover.
    ///
    /// This exists for [`crate::deck::Frame`], which calls `begin_frame` on
    /// every slot at the top of a frame and then has to reach those same Sets
    /// again once its encoder is open. Splitting the two is safe there
    /// precisely because the deck's guard holds the encoder: between the
    /// install and this call there is no point at which a caller could have
    /// obtained a different Set.
    ///
    /// `pub(crate)`, and it should stay that way. Handing this out publicly
    /// would be a second way to render a Set, next to the one that installs
    /// builds on a frame boundary, and "which Sets is this frame made of"
    /// would stop having one answer.
    pub(crate) fn live_mut(&mut self) -> &mut Set {
        &mut self.live
    }

    /// Frames begun since construction. What "a build does not block the
    /// render loop" is measured in: the count between a request going out and
    /// its [`Event::Swapped`] coming back is how many frames were produced
    /// while the worker was busy.
    pub fn frames_rendered(&self) -> u64 {
        self.frames
    }

    /// The budget the watchdog is holding candidates to, in milliseconds.
    pub fn budget_ms(&self) -> f32 {
        self.budget_ms
    }

    /// **What one frame of the live Set costs**, as measured when it was built
    /// — not a frame interval, and not this frame.
    ///
    /// `None` means nothing has measured this Set, which is not the same as
    /// "it is free": [`HotSwap::fixed`] and the Set [`HotSwap::new`] is
    /// constructed with arrive unmeasured, and stay that way until
    /// [`HotSwap::measure_live`] is called. [`crate::governor`] treats an
    /// unmeasured Set as unbudgetable rather than as zero, which is the only
    /// safe reading — **including when the slot is Live**, where it makes the
    /// deck's committed cost unknown and suspends priming. Measuring the Sets
    /// a caller built itself is therefore a startup step rather than an
    /// optional refinement; see
    /// [`Deck::measure_slots`](crate::deck::Deck::measure_slots).
    ///
    /// Read [`Measurement::method`] before trusting the number; read
    /// "The worker also measures what it built" in the module doc for what it
    /// is a measurement of and what it cannot see.
    pub fn measured_cost(&self) -> Option<Measurement> {
        self.cost
    }

    /// **Measure the Set that is live now**, and keep the result.
    ///
    /// For a Set no worker built — [`HotSwap::fixed`]'s, or the one handed to
    /// [`HotSwap::new`] — which is otherwise invisible to the governor, and
    /// which makes the whole deck unbudgetable for as long as it is on air and
    /// unmeasured. This is the reachable form of [`measure`]: the live `Set` is
    /// not handed out mutably (that is `live_mut`, and it is `pub(crate)` for
    /// good reasons), so without this a caller cannot measure a Set it has
    /// already put into a `HotSwap`.
    ///
    /// **Never on the render thread and never inside a frame.** It submits and
    /// waits, once per sample, and it steps the Set and rewinds it — which is
    /// another whole-capacity upload. Startup is where it belongs.
    ///
    /// **On a Set that has already stepped, this is destructive.** [`measure`]
    /// ends in [`Set::rewind`], which restores what `Set::build` left rather
    /// than what this call found: `t` back to zero, element buffers cold. That
    /// is exactly right for the cold Set it is meant for and is a reset for any
    /// other, so measure before the first frame.
    /// [`Deck::measure_slots`](crate::deck::Deck::measure_slots) skips a slot
    /// that has stepped for this reason; a caller reaching this directly is
    /// holding the check itself.
    ///
    /// `probe` is taken rather than constructed for the reason [`measure`]
    /// gives: calibration is expensive and two probes can disagree about this
    /// adapter, producing numbers a governor would then be summing.
    /// [`Deck::measure_slots`](crate::deck::Deck::measure_slots) is the
    /// deck-wide version and constructs one probe for all of them.
    pub fn measure_live(
        &mut self,
        probe: &mut Probe,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Measurement {
        let cost = measure(probe, device, queue, &mut self.live);
        self.cost = Some(cost);
        cost
    }

    /// Attach a measurement to the Set that is live now.
    ///
    /// [`HotSwap::measure_live`] is the one that takes it; this is for a caller
    /// that has a number from somewhere else — a test pinning exact
    /// arithmetic, or a measurement carried across a rebuild of the same Set.
    ///
    /// Replaced wholesale by the next build that lands, since that
    /// measurement is of a different Set.
    pub fn set_measured_cost(&mut self, cost: Measurement) {
        self.cost = Some(cost);
    }

    /// Whether a candidate is currently on trial. While one is, incoming
    /// builds are left in the channel — see [`HotSwap::install_if_ready`].
    pub fn on_trial(&self) -> bool {
        self.trial.is_some()
    }

    /// Everything that has happened since this was last called.
    pub fn events(&mut self) -> std::vec::Drain<'_, Event> {
        self.events.drain(..)
    }

    /// The same events, read without taking them.
    ///
    /// [`HotSwap::events`] is the caller's — it drains, because a caller that
    /// reads an event twice would print a rollback twice. Something that has
    /// to *react* to a swap rather than report it cannot use that without
    /// stealing it, so this is the read-only view: `Deck::begin_frame` notes
    /// the length before the frame boundary and looks at what was appended,
    /// which is how it knows a build landed on a slot and its level meter is
    /// now measuring different material. It changes nothing and consumes
    /// nothing.
    pub fn pending_events(&self) -> &[Event] {
        &self.events
    }

    /// Remembered as well as forwarded: a Set built while the window was one
    /// size must not arrive on screen still believing it, and the parked Set
    /// must not come back through a rollback with a stale aspect ratio.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.viewport = (width, height);
        self.live.resize(device, width, height);
        if let Some(previous) = &mut self.previous {
            previous.resize(device, width, height);
        }
    }

    /// Feed the watchdog one frame interval, and act on it if the window is
    /// full.
    fn record(&mut self, ms: f32) {
        let Some(trial) = &mut self.trial else {
            return;
        };
        trial.seen += 1;
        if trial.seen <= WARMUP_FRAMES {
            return;
        }
        self.samples.push(ms);
        if self.samples.len() < JUDGE_FRAMES {
            return;
        }

        self.samples
            .sort_by(|a, b| a.partial_cmp(b).expect("frame intervals are finite"));
        let median_ms = self.samples[self.samples.len() / 2];
        let trial = self.trial.take().expect("checked above");
        let previous = self
            .previous
            .take()
            .expect("a trial is only ever started with a rollback target in hand");

        if median_ms > self.budget_ms {
            // The candidate goes, the parked Set comes back. It resumes at the
            // `t` it stopped at, because nothing stepped it while it waited —
            // and its measurement comes back with it, or the governor would go
            // on budgeting for the candidate that is no longer there.
            let candidate = std::mem::replace(&mut self.live, previous);
            self.cost = self.previous_cost.take();
            self.retire(candidate);
            self.events.push(Event::RolledBack {
                id: trial.id,
                label: trial.label,
                median_ms,
                budget_ms: self.budget_ms,
            });
        } else {
            self.previous_cost = None;
            self.retire(previous);
            self.events.push(Event::Accepted {
                id: trial.id,
                label: trial.label,
                median_ms,
                budget_ms: self.budget_ms,
            });
        }
    }

    /// Install a finished build, if one is waiting and there is room for it.
    fn install_if_ready(&mut self, device: &wgpu::Device) {
        // Not while something is on trial: `previous` is the rollback target
        // and there is exactly one of it, so accepting a second candidate
        // would mean losing the only Set known to work. A build that finishes
        // during a trial stays in the channel until the verdict is in.
        if self.trial.is_some() {
            return;
        }
        // `try_recv`, never `recv`, and drained to the *newest* result rather
        // than stopping at the first. An `mpsc` channel is FIFO and a judging
        // window is long enough for two saves to finish behind it, so taking
        // the front of the queue would put a superseded Set on screen for a
        // whole window before reaching the one the operator is waiting for.
        // A superseded error is still reported — a diagnostic is the point of
        // an error, superseded or not — and a superseded Set is retired to the
        // worker rather than dropped here.
        let mut newest: Option<Built> = None;
        loop {
            match self.built.try_recv() {
                Ok(next) => {
                    if let Some(stale) = newest.replace(next) {
                        match stale.result {
                            Ok(set) => self.retire(set),
                            Err(error) => self.events.push(Event::Rejected {
                                id: stale.id,
                                label: stale.label,
                                error,
                            }),
                        }
                    }
                }
                // `Empty` and `Disconnected` are the same thing from here: no
                // Set to install, so render and move on. A worker that is gone
                // while one was expected is reported once, below.
                Err(mpsc::TryRecvError::Disconnected) => {
                    if self.worker.is_some() && !self.worker_lost {
                        self.worker_lost = true;
                        self.events.push(Event::WorkerLost);
                    }
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }
        let Some(built) = newest else {
            return;
        };

        match built.result {
            Err(error) => self.events.push(Event::Rejected {
                id: built.id,
                label: built.label,
                error,
            }),
            Ok(mut candidate) => {
                candidate.resize(device, self.viewport.0, self.viewport.1);
                let outgoing = std::mem::replace(&mut self.live, candidate);
                self.previous = Some(outgoing);
                self.previous_cost = std::mem::replace(&mut self.cost, built.cost);
                self.samples.clear();
                self.trial = Some(Trial {
                    id: built.id,
                    label: Arc::clone(&built.label),
                    seen: 0,
                });
                self.events.push(Event::Swapped {
                    id: built.id,
                    label: built.label,
                });
            }
        }
    }

    /// Queue a Set for the worker to drop. Never drops it here.
    fn retire(&mut self, set: Set) {
        self.retired.push(set);
        self.hand_over_retired();
    }

    /// `try_lock`, never `lock`: the render thread waits on the worker for
    /// nothing. A Set that misses this frame's handover is handed over next
    /// frame instead, and holding one for a few extra frames costs only the
    /// memory it was already using.
    fn hand_over_retired(&mut self) {
        if self.retired.is_empty() {
            return;
        }
        if let Ok(mut graveyard) = self.graveyard.try_lock() {
            graveyard.append(&mut self.retired);
        }
    }
}

impl Drop for HotSwap {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            // Up to one `POLL_INTERVAL`, because the worker is most likely
            // sitting inside `Source::poll`.
            let _ = worker.join();
        }
    }
}

/// How many Sets the handover between the render thread and the worker is
/// sized for. At most one Set is retired per verdict and verdicts are
/// `JUDGE_FRAMES` apart, so two is already slack; the capacity exists so that
/// the render thread's `push` and `append` never have to grow anything.
const GRAVEYARD_CAPACITY: usize = 4;

/// Likewise for events: at most one install and one verdict can happen in a
/// frame, and they are mutually exclusive.
const EVENT_CAPACITY: usize = 4;

/// Measure one frame of `set` and leave it exactly as it was found.
///
/// **Never on the render thread.** It submits and waits, once per sample, which
/// is what makes the number mean anything and is the pattern the frame loop must
/// never use; and it steps the Set, so it has to rewind it afterwards, which is
/// another whole-capacity upload. The worker calls this as part of building; a
/// caller with a [`HotSwap::fixed`] Set reaches it through
/// [`HotSwap::measure_live`], or through
/// [`Deck::measure_slots`](crate::deck::Deck::measure_slots) for a whole deck
/// at once, before the first frame.
///
/// `probe` is taken rather than constructed because constructing one runs a
/// calibration workload ten times over (see [`Probe::new`]), and because two
/// independently constructed probes can land on different
/// [`MeasurementMethod`](crate::probe::MeasurementMethod)s and produce numbers
/// that are not comparable — which is precisely what a governor summing them
/// would then be doing.
///
/// The Set is resized to [`PROBE_RESOLUTION`] for the run and **resized back**
/// before this returns. That is not decoration: the viewport is what the
/// camera's aspect ratio is derived from in [`Set::prepare`], it is the one
/// piece of a Set's state a probe run touches that [`Set::rewind`] has no
/// business resetting — `build` leaves it at 1×1 and a caller's size is not a
/// thing to rewind to — and a Set left at 720p renders a different picture on
/// its first frame in a deck of any other shape. `HotSwap::install_if_ready`
/// happens to resize an arriving candidate anyway, which is what kept this
/// invisible; a caller measuring its own [`HotSwap::fixed`] Set has no such
/// second chance.
pub fn measure(
    probe: &mut Probe,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    set: &mut Set,
) -> Measurement {
    let capacity = set.capacity();
    let viewport = set.viewport();
    set.resize(device, PROBE_RESOLUTION.0, PROBE_RESOLUTION.1);
    // The uniforms have never been written otherwise — `build` allocates them
    // and leaves them at whatever the driver's fresh buffer holds — so the
    // measured frame has to be preceded by a real `prepare`, exactly as an
    // on-air frame is. The session's signals are `default` here: a probe run is
    // not part of a session and has no tick sequence of its own, and a binding
    // resolved against a phase of zero is the same shape of work as one
    // resolved against any other phase.
    set.prepare(queue, PROBE_STEPS, &Signals::default());
    let measurement = probe.run(device, queue, set, PROBE_STEPS, capacity);
    // Back to cold. `Set::rewind` is documented for this one caller.
    set.rewind(device, queue);
    set.resize(device, viewport.0, viewport.1);
    measurement
}

/// The build worker.
///
/// A poll loop rather than a blocking one, because it has two jobs: asking the
/// source for work, and freeing whatever the render thread retired. A `recv`
/// that blocked until the next request would leave a rolled-back Set's buffers
/// resident until the operator happened to edit a file again.
fn run_worker(
    device: wgpu::Device,
    queue: wgpu::Queue,
    mut source: Box<dyn Source>,
    out: Sender<Built>,
    graveyard: Arc<Mutex<Vec<Set>>>,
    stop: Arc<AtomicBool>,
) {
    // Constructed once, on first use, and reused for every build this worker
    // ever does. Once because calibration is expensive (ten heavy runs) and
    // once because two probes can disagree with each other about whether this
    // adapter's timestamps work — see `Probe::new`. Lazily because a worker
    // that is never given anything to build should not allocate a 720p target
    // and half a second of calibration for nothing.
    let mut probe: Option<Probe> = None;

    while !stop.load(Ordering::Relaxed) {
        // Take the retired Sets out from under the lock before dropping them:
        // releasing a Set's GPU resources is not instant, and the render
        // thread's `try_lock` would fail for the whole of it. `drain` leaves
        // the graveyard's capacity intact, which is what keeps the render
        // thread's side allocation-free.
        let condemned: Vec<Set> = match graveyard.lock() {
            Ok(mut held) => held.drain(..).collect(),
            // Poisoned: the render thread panicked holding it. There is
            // nothing to recover here and the process is going down anyway.
            Err(_) => Vec::new(),
        };
        // `held` is gone by here, so the lock is free while the expensive part
        // — actually releasing the GPU resources — happens.
        drop(condemned);

        let Some(request) = source.poll() else {
            continue;
        };
        let id = request.id;
        let label: Arc<str> = request.label.into();

        // Caught, not allowed to propagate. A panic here would take the whole
        // worker with it, and the render thread's only symptom would be that
        // nothing is ever built again — no error, no event, just a `--watch`
        // that quietly stops responding to saves. That is the worst shape a
        // failure can take on stage. `Set::build` creates shader modules, and
        // wgpu's default handler for an uncaptured validation error is a
        // panic, so this is reachable from any generated WGSL naga refuses:
        // a compiler bug, but one that must not be an unexplained silence.
        let build = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Set::build_many(
                &device,
                &queue,
                &request.l1s.iter().map(|(p, c)| (p, *c)).collect::<Vec<_>>(),
                &request.l2s.iter().collect::<Vec<_>>(),
                request.l3.as_ref(),
                request.field.as_ref(),
                &request.l4s.iter().collect::<Vec<_>>(),
                request.layering,
                request.seed_salt,
            )
            .map(|mut set| {
                for write in &request.params {
                    // Addressed or not — `Set::write_param` is the one place
                    // that decides, so this path and the command line's cannot
                    // disagree about what a bare name means.
                    if set.write_param(write) == 0 {
                        eprintln!("  no parameter named `{}`, ignoring", write.key);
                    }
                }
                // **Before the bindings**, because a macro is a binding whose
                // source is a published control: a binding attached before the
                // control existed would be attached to a name nothing answers,
                // and would hold its param where it found it for the rest of
                // the run.
                for control in request.published {
                    let name = control.name.clone();
                    if let Err(e) = set.publish(control) {
                        eprintln!("  `{name}` is not published: {e}");
                    }
                }
                // After the params, because a binding blends from a param's
                // value: applying them the other way round would leave the
                // first frame after a swap blending from the `.kir` default.
                for binding in request.bindings {
                    let (layer, key) = (binding.layer, binding.key.clone());
                    if !set.bind(binding) {
                        eprintln!("  no {layer:?} parameter named `{key}` to bind, ignoring");
                    }
                }
                set
            })
        }));
        let result = match build {
            Ok(result) => result,
            Err(payload) => Err(SetError::Panicked {
                label: label.to_string(),
                detail: panic_detail(&payload),
            }),
        };

        let mut result = result;
        let mut cost = None;
        if let Ok(set) = &mut result {
            // `Set::build` leaves the whole element and alive buffer contents
            // staged on the queue — megabytes at a real capacity. Left there,
            // they would be flushed by whatever the *render thread* submits
            // next, putting the upload inside the first frame the swap was
            // supposed to be invisible to. Flushing it here gives it its own
            // submission, and waiting for it here means the Set is resident
            // before it is handed over. This is the submit-and-wait the render
            // thread must never do; a worker thread is exactly where it
            // belongs.
            queue.submit([]);
            let _ = device.poll(wgpu::PollType::Wait);

            // And then measure it, on the same thread and for the same reason.
            // Caught for the same reason the build is: this reaches driver code
            // through a freshly generated pipeline, and a panic here would take
            // the worker with it and leave a `--watch` session silently
            // unable to build anything again. A build that could not be
            // measured is still a build — it travels without a measurement and
            // the governor declines to budget for it, which is the conservative
            // reading rather than a failure.
            let probe = probe.get_or_insert_with(|| {
                Probe::new(
                    &device,
                    &queue,
                    device.features().contains(wgpu::Features::TIMESTAMP_QUERY),
                    PROBE_RESOLUTION,
                )
            });
            cost = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                measure(probe, &device, &queue, set)
            }))
            .ok();
            if cost.is_none() {
                eprintln!("  `{label}` could not be measured; it will not be budgeted for");
            }

            // And flush again, for the reason the first flush exists. `measure`
            // ends in `Set::rewind`, which re-stages the *whole* element and
            // alive buffers — the same megabytes `build` staged, put back a
            // second time — and nothing in `measure` submits after it. Left
            // here they would ride out on the render thread's next submission,
            // which is precisely the cost the flush above was added to keep out
            // of the swap frame. The flush before the measurement does not
            // cover an upload the measurement itself creates.
            queue.submit([]);
            let _ = device.poll(wgpu::PollType::Wait);
        }

        if out.send(Built {
            id,
            label,
            result,
            cost,
        })
        .is_err()
        {
            // The render thread is gone.
            break;
        }
    }
}

/// The message out of a caught panic, which arrives as a `Box<dyn Any>` and is
/// a `&str` or a `String` for every panic the standard macros produce.
fn panic_detail(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "no message".to_string()
    }
}

/// `Set` must stay `Send`, because the whole design above rests on building
/// one on a worker thread and moving it to the render thread.
///
/// It is today: every field is a wgpu handle (all `Send + Sync` in wgpu 26),
/// a `Vec`, a `HashMap`, or a plain number. Nothing is a `Rc`, a `Cell`, or a
/// raw pointer. This assertion is here so that a field which *is* one of those
/// fails to compile at the line that explains why it matters, rather than
/// somewhere inside `HotSwap::new` where the fix looks like "wrap it in a
/// mutex" instead of "do not put that in a Set".
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<Set>();
    assert_send::<Request>();
};
