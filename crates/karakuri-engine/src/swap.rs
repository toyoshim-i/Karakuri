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
//! ## Why a swap cannot land mid-frame
//!
//! [`HotSwap::begin_frame`] is the only place `live` is replaced, and it
//! returns `&mut Set` borrowed from `&mut self`. The caller holds that borrow
//! for as long as it is recording the frame, and while it holds it the borrow
//! checker will not let it call `begin_frame` — or anything else on the
//! `HotSwap` — again. So "a frame renders entirely with the old Set or
//! entirely with the new one" is not a claim about when the code happens to
//! call what; it is the only shape the code compiles in. A swap in the middle
//! of an encoder would require two overlapping `&mut` borrows of the same
//! `HotSwap`.
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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use karakuri_ir::typed::Checked;

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

/// How long a [`Source`] with nothing to report should block before returning.
/// The worker does nothing else between polls, so a source that returns
/// immediately turns it into a spin.
pub const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Everything the worker needs to build one Set. Owned, not borrowed: the
/// worker is on another thread and cannot hold anything belonging to the
/// render loop.
pub struct Request {
    pub l1: Checked,
    pub l4: Checked,
    pub capacity: u32,
    pub seed_salt: u32,
    /// Applied to the new Set once it is built. Parameter values are the one
    /// piece of Set state that is not structural, so they are the one thing
    /// worth carrying across a swap — and they are carried by being *restated*
    /// here rather than read out of the outgoing Set, because a request that
    /// depends on what happens to be live is not reproducible from a record
    /// stream.
    pub params: Vec<(String, f32)>,
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
    Swapped { label: Arc<str> },
    /// A build failed. **Nothing changed**: the running Set is still running,
    /// with its `t` and its live count untouched.
    Rejected { label: Arc<str>, error: SetError },
    /// The watchdog's verdict, in favour. The previous Set is released.
    Accepted { label: Arc<str>, median_ms: f32 },
    /// The watchdog's verdict, against. The previous Set is live again, at the
    /// `t` it was parked at, and the candidate is released.
    RolledBack {
        label: Arc<str>,
        median_ms: f32,
        budget_ms: f32,
    },
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Swapped { label } => {
                write!(f, "swapped in `{label}` — cold, t back to zero")
            }
            Event::Rejected { label, error } => {
                write!(f, "`{label}` was refused, nothing changed:\n{error}")
            }
            Event::Accepted { label, median_ms } => write!(
                f,
                "`{label}` held the budget ({median_ms:.2} ms median frame interval, host clock)"
            ),
            Event::RolledBack {
                label,
                median_ms,
                budget_ms,
            } => write!(
                f,
                "rolled back `{label}`: {median_ms:.2} ms median frame interval over \
                 {JUDGE_FRAMES} frames exceeds the {budget_ms:.2} ms budget \
                 (host clock — see README's Working style)"
            ),
        }
    }
}

/// A finished build on its way back to the render thread.
struct Built {
    label: Arc<str>,
    result: Result<Set, SetError>,
}

/// The candidate currently being watched.
struct Trial {
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
    /// The Set that was live before the current one, held so that a rollback
    /// is a move rather than a rebuild. Not stepped while parked; see "What a
    /// swap does not do" in the module doc.
    previous: Option<Set>,
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
            previous: None,
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
        }
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
            previous: None,
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
        }
    }

    /// The top of a frame, and the only place the live Set is ever replaced.
    ///
    /// Everything that can change which Set is live happens here, before the
    /// borrow is handed out: the previous frame's interval is fed to the
    /// watchdog (which may roll back), and then a finished build is installed
    /// if one has arrived. The caller records its whole frame through the
    /// returned reference, and cannot touch this `HotSwap` again until it
    /// drops it — which is why a swap cannot land mid-encoder. See "Why a swap
    /// cannot land mid-frame" in the module doc.
    ///
    /// Allocates nothing, compiles nothing, and never blocks: the channel is
    /// polled with `try_recv` and the graveyard with `try_lock`.
    pub fn begin_frame(&mut self) -> &mut Set {
        let now = Instant::now();
        if let Some(last) = self.last_frame.replace(now) {
            // The interval that just ended is the *previous* frame's duration,
            // so the watchdog is always one frame behind. It has to be: a
            // frame's cost is not known until the next one starts.
            self.record(now.duration_since(last).as_secs_f32() * 1_000.0);
        }
        self.frames += 1;
        self.hand_over_retired();
        self.install_if_ready();
        &mut self.live
    }

    /// The live Set, outside a frame. Read-only, so it cannot be rendered
    /// through — recording a frame goes through [`HotSwap::begin_frame`], and
    /// that is deliberate.
    pub fn set(&self) -> &Set {
        &self.live
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

    /// Whether a candidate is currently on trial. While one is, incoming
    /// builds are left in the channel — see [`HotSwap::install_if_ready`].
    pub fn on_trial(&self) -> bool {
        self.trial.is_some()
    }

    /// Everything that has happened since this was last called.
    pub fn events(&mut self) -> std::vec::Drain<'_, Event> {
        self.events.drain(..)
    }

    /// Remembered as well as forwarded: a Set built while the window was one
    /// size must not arrive on screen still believing it, and the parked Set
    /// must not come back through a rollback with a stale aspect ratio.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.viewport = (width, height);
        self.live.resize(width, height);
        if let Some(previous) = &mut self.previous {
            previous.resize(width, height);
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
            // `t` it stopped at, because nothing stepped it while it waited.
            let candidate = std::mem::replace(&mut self.live, previous);
            self.retire(candidate);
            self.events.push(Event::RolledBack {
                label: trial.label,
                median_ms,
                budget_ms: self.budget_ms,
            });
        } else {
            self.retire(previous);
            self.events.push(Event::Accepted {
                label: trial.label,
                median_ms,
            });
        }
    }

    /// Install a finished build, if one is waiting and there is room for it.
    fn install_if_ready(&mut self) {
        // Not while something is on trial: `previous` is the rollback target
        // and there is exactly one of it, so accepting a second candidate
        // would mean losing the only Set known to work. A build that finishes
        // during a trial simply stays in the channel until the verdict is in,
        // which is also what a file watcher wants — the build still waiting is
        // the newer one.
        if self.trial.is_some() {
            return;
        }
        // `try_recv`, never `recv`. `Empty` and `Disconnected` are the same
        // thing from here: no Set to install, so render and move on.
        let Ok(built) = self.built.try_recv() else {
            return;
        };

        match built.result {
            Err(error) => self.events.push(Event::Rejected {
                label: built.label,
                error,
            }),
            Ok(mut candidate) => {
                candidate.resize(self.viewport.0, self.viewport.1);
                let outgoing = std::mem::replace(&mut self.live, candidate);
                self.previous = Some(outgoing);
                self.samples.clear();
                self.trial = Some(Trial {
                    label: Arc::clone(&built.label),
                    seen: 0,
                });
                self.events.push(Event::Swapped { label: built.label });
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
        let label: Arc<str> = request.label.into();

        let result = Set::build(
            &device,
            &queue,
            &request.l1,
            &request.l4,
            request.capacity,
            request.seed_salt,
        )
        .map(|mut set| {
            for (name, value) in request.params {
                match set.params.get_mut(&name) {
                    Some(slot) => *slot = value,
                    None => eprintln!("  no parameter named `{name}`, ignoring"),
                }
            }
            set
        });

        if result.is_ok() {
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
        }

        if out.send(Built { label, result }).is_err() {
            // The render thread is gone.
            break;
        }
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
