//! Hot swap coordination and watchdog budgeting.
//!
//! Hot swapping compiles and validates incoming `Set` pipelines on a background
//! worker thread, transfers finished Sets across a channel, and atomically installs
//! them at frame boundaries without stalling the render loop.
//!
//! # Architecture
//!
//! - **Background Compilation**: Shaders, pipelines, and element buffers are created
//!   and initialized on a worker thread using cloned `wgpu::Device` and `wgpu::Queue`
//!   handles. The render thread polls with `try_recv` and never blocks.
//! - **Frame Boundary Installation**: Swaps occur exclusively at the beginning of a
//!   frame via [`HotSwap::begin_frame`]. Live Sets are never replaced mid-frame.
//! - **State Reset**: Incoming Sets arrive cold, with simulation time reset to zero
//!   and fresh element buffers. Parameter adjustments and bound inputs are carried
//!   over from the outgoing Set via [`Set::carry_moved_from`] and [`Set::carry_bound_from`].
//! - **Watchdog Budgeting**: Candidate Sets are measured on the worker thread prior
//!   to handover. If candidate frame execution time exceeds the target budget,
//!   the slot is flagged as overloaded and frozen on its current still to protect
//!   the render loop from frame drops.
//! - **Asynchronous Deallocation**: Retired Sets are queued into a graveyard mutex
//!   and dropped by the background worker, preventing GPU resource deallocations
//!   from stalling the render thread.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;

use crate::binding::{Binding, Signals};
use crate::estimate::{estimate, Estimate};
use crate::governor::{self, Basis, Estimated};
use crate::probe::{Measurement, Probe};
use crate::set::{Authority, Set, SetError};

/// Number of frames over which the rolling frame period is measured.
pub const PERIOD_FRAMES: usize = 30;

/// Default frame budget in milliseconds when display refresh rate is unknown.
///
/// Corresponds to one 60 Hz frame (16.67 ms) plus allowance for host-clock
/// measurement overhead.
pub const DEFAULT_BUDGET_MS: f32 = 20.0;

/// Simulation steps per measured probe frame.
pub const PROBE_STEPS: u8 = 1;

/// Duration for worker thread polling when no source requests are pending.
pub const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Procedural node names for a build request.
#[derive(Debug, Clone, Default)]
pub struct RequestNames {
    pub l1s: Vec<Option<String>>,
    pub l2s: Vec<Option<String>>,
    /// Camera node names, including the built-in orbit camera.
    pub l3s: Vec<Option<String>>,
    pub l4s: Vec<Option<String>>,
    /// Field node names declared in the set definition.
    pub fields: Vec<Option<String>>,
}

/// Specification for building a Set on the background worker thread.
pub struct Request {
    /// Caller-provided identifier for tracking build outcomes across events.
    pub id: u64,
    /// Geometry sources paired with their respective element capacities.
    pub l1s: Vec<(Checked, u32)>,
    /// Deformation stages in chain execution order.
    pub l2s: Vec<Checked>,
    /// Camera definitions in node order.
    pub l3s: Vec<Checked>,
    /// Field procedures in node order.
    pub fields: Vec<Checked>,
    /// Renderer procedures in draw order.
    pub l4s: Vec<Checked>,
    /// Layering strategy determining whether renderers overdraw or composite.
    pub layering: crate::set::Layering,
    /// Optional renderer index to isolate. When `None`, all renderers remain active.
    pub live: Option<u32>,
    /// Base seed salt for procedural noise and hashing.
    pub seed_salt: u32,
    /// Explicit hash salts assigned per geometry source.
    pub salts: Vec<Option<u32>>,
    /// Built-in orbit camera initial configuration.
    pub camera: crate::camera::Orbit,
    /// Parameter overrides explicitly requested for this build.
    pub params: Vec<crate::binding::ParamWrite>,
    /// Interface controls exposed by this Set.
    pub published: Vec<crate::set::Published>,
    /// Input signal bindings attached to Set parameters.
    pub bindings: Vec<Binding>,
    /// Assigned node names across layers.
    pub names: RequestNames,
    /// Edge connections routing data between node input and output slots.
    pub edges: Vec<crate::set::Edge>,
    /// Explicit node authority assignments.
    pub authorities: Vec<AuthorityAt>,
    /// Human-readable label identifying this build request.
    pub label: String,
}

/// Explicit authority level assigned to a specific node address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorityAt {
    /// Node address represented as `(layer, index)`.
    pub at: (Kind, u32),
    /// Authority level granted to the node.
    pub authority: Authority,
}

impl AuthorityAt {
    pub fn new(layer: Kind, index: u32, authority: Authority) -> AuthorityAt {
        AuthorityAt {
            at: (layer, index),
            authority,
        }
    }
}

/// Diagnostic details when source material is rejected before compilation.
pub struct Refusal {
    /// Identifier or path of the rejected source.
    pub label: String,
    /// Diagnostic error messages in order of emission.
    pub said: Vec<String>,
}

/// Result of polling a build source.
#[allow(clippy::large_enum_variant)]
pub enum Polled {
    /// Source material ready to be compiled into a Set.
    Build(Request),
    /// Source material failed validation and was not compiled.
    Refused(Refusal),
}

/// Source providing build requests or validation refusals to the worker thread.
pub trait Source: Send {
    /// Polls for pending work. Blocks for approximately [`POLL_INTERVAL`] when idle.
    fn poll(&mut self) -> Option<Polled>;
}

impl Source for Receiver<Request> {
    fn poll(&mut self) -> Option<Polled> {
        match self.recv_timeout(POLL_INTERVAL) {
            Ok(request) => Some(Polled::Build(request)),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => {
                std::thread::sleep(POLL_INTERVAL);
                None
            }
        }
    }
}

/// Events emitted across the hot swap lifecycle.
#[derive(Debug)]
pub enum Event {
    /// Build completed and was installed as the live Set.
    Swapped { id: u64, label: Arc<str> },
    /// Build failed compilation or validation. The previous Set remains active.
    Rejected {
        id: u64,
        label: Arc<str>,
        error: SetError,
    },
    /// Source material was refused before a build was attempted.
    SourceRefused { label: Arc<str>, said: Vec<String> },
    /// Installed candidate was accepted by the watchdog and remains active.
    Accepted {
        id: u64,
        label: Arc<str>,
        /// Measured cost of one candidate frame, or `None` if unmeasured.
        cost_ms: Option<f32>,
        /// Measurement or estimation basis used for budgeting.
        basis: Basis,
        /// Frame budget against which the candidate was evaluated.
        budget_ms: f32,
    },
    /// Candidate exceeded the frame budget; slot is frozen to prevent frame drops.
    Overloaded {
        id: u64,
        label: Arc<str>,
        /// Candidate frame duration in milliseconds that exceeded `budget_ms`.
        cost_ms: f32,
        /// Measurement basis for the cost reading.
        basis: Basis,
        /// Frame budget that was exceeded.
        budget_ms: f32,
    },
    /// Worker thread terminated unexpectedly.
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
            Event::SourceRefused { label, said } => write!(
                f,
                "`{label}` did not compile, nothing was built:\n{}",
                said.join("\n")
            ),
            Event::Accepted {
                label,
                id: _,
                cost_ms: Some(cost_ms),
                basis,
                budget_ms,
            } => write!(
                f,
                "`{label}` held the budget: {cost_ms:.2} ms for its own frame \
                 against {budget_ms:.2} ms ({})",
                said(*basis)
            ),
            Event::Accepted {
                label,
                id: _,
                cost_ms: None,
                basis: _,
                budget_ms,
            } => write!(
                f,
                "`{label}` was not judged and stays: nothing measured it and nothing \
                 estimated it, so there is no number to hold against {budget_ms:.2} ms \
                 (see docs/principles/0084-…)"
            ),
            Event::Overloaded {
                label,
                id: _,
                cost_ms,
                basis,
                budget_ms,
            } => write!(
                f,
                "`{label}` is overloaded: {cost_ms:.2} ms for its own frame exceeds the \
                 {budget_ms:.2} ms budget ({} — see docs/contributing.md, working style). \
                 It is still in the slot and the slot has stopped updating; a fader to \
                 zero, an earlier version, or a build that fits is what ends it",
                said(*basis)
            ),
            Event::WorkerLost => write!(
                f,
                "the build worker is gone; nothing more will be built this run \
                 (the live Set is unaffected)"
            ),
        }
    }
}

/// Returns a human-readable description of the measurement basis.
fn said(basis: Basis) -> &'static str {
    match basis {
        Basis::Measured => "one draw at the size the caller named, host clock",
        Basis::Estimated => "a two-draw fit at the output's size",
        Basis::Unbudgetable => "no number",
    }
}

/// Worker response containing either a finished build or a pre-build refusal.
#[allow(clippy::large_enum_variant)]
enum Done {
    Built(Built),
    Refused(Refusal),
}

/// Finished build result ready for installation on the render thread.
struct Built {
    id: u64,
    label: Arc<str>,
    result: Result<Set, SetError>,
    cost: Option<Measurement>,
}

/// Rolling median frame period measured across successive frame boundaries.
struct Period {
    samples: [f32; PERIOD_FRAMES],
    next: usize,
    filled: bool,
    last: Option<Instant>,
}

impl Period {
    fn new() -> Period {
        Period {
            samples: [0.0; PERIOD_FRAMES],
            next: 0,
            filled: false,
            last: None,
        }
    }

    /// Records a frame boundary timestamp and updates the rolling window.
    fn mark(&mut self, now: Instant) {
        if let Some(last) = self.last.replace(now) {
            self.samples[self.next] = now.duration_since(last).as_secs_f32() * 1_000.0;
            self.next = (self.next + 1) % PERIOD_FRAMES;
            self.filled |= self.next == 0;
        }
    }

    /// Returns the rolling median frame period in milliseconds, or `None` if window is not full.
    fn median_ms(&self) -> Option<f32> {
        if !self.filled {
            return None;
        }
        let mut sorted = self.samples;
        sorted.sort_by(|a, b| a.partial_cmp(b).expect("frame intervals are finite"));
        Some(sorted[PERIOD_FRAMES / 2])
    }
}

/// Manages a live Set, background worker compilation, and watchdog budgeting.
pub struct HotSwap {
    live: Set,
    /// Measured cost of the currently live Set, if measured.
    cost: Option<Measurement>,
    /// Estimated cost of the live Set at output resolution, if computed.
    estimate: Option<Estimate>,
    /// Frame budget in milliseconds against which candidate Sets are evaluated.
    budget_ms: f32,
    /// Whether the live Set has been stopped due to exceeding the frame budget.
    overloaded: bool,
    viewport: (u32, u32),
    /// Rolling median frame period measurement.
    period: Period,
    frames: u64,
    /// Accumulator for hot swap lifecycle events.
    events: Vec<Event>,

    /// Receiver for completed builds or validation refusals from the worker.
    done: Receiver<Done>,
    /// Target resolution for probe measurements, packed as `(width << 32) | height`.
    measure_at: Arc<AtomicU64>,
    graveyard: Arc<Mutex<Vec<Set>>>,
    /// Sets retired by the render thread pending handover to the worker graveyard.
    retired: Vec<Set>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    /// Whether worker channel disconnection has already been reported.
    worker_lost: bool,
}

impl HotSwap {
    /// Creates a new `HotSwap` coordinator backed by a background worker thread.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        live: Set,
        budget_ms: f32,
        source: Box<dyn Source>,
    ) -> HotSwap {
        let (done_tx, done_rx) = mpsc::channel();
        let graveyard = Arc::new(Mutex::new(Vec::with_capacity(GRAVEYARD_CAPACITY)));
        let stop = Arc::new(AtomicBool::new(false));
        let measure_at = Arc::new(AtomicU64::new(packed(live.viewport())));

        let worker = {
            let device = device.clone();
            let queue = queue.clone();
            let graveyard = Arc::clone(&graveyard);
            let stop = Arc::clone(&stop);
            let measure_at = Arc::clone(&measure_at);
            std::thread::Builder::new()
                .name("karakuri-build".into())
                .spawn(move || {
                    run_worker(device, queue, source, done_tx, graveyard, stop, measure_at)
                })
                .expect("spawn build worker")
        };

        HotSwap {
            live,
            cost: None,
            estimate: None,
            budget_ms,
            overloaded: false,
            viewport: (1, 1),
            period: Period::new(),
            frames: 0,
            events: Vec::with_capacity(EVENT_CAPACITY),
            done: done_rx,
            measure_at,
            graveyard,
            retired: Vec::with_capacity(GRAVEYARD_CAPACITY),
            stop,
            worker: Some(worker),
            worker_lost: false,
        }
    }

    /// Installs a Set immediately without background compilation or budgeting.
    ///
    /// Used during replay runs where operations and procedure versions are pre-recorded.
    pub fn install(&mut self, device: &wgpu::Device, mut set: Set) {
        set.resize(device, self.viewport.0, self.viewport.1);
        let outgoing = std::mem::replace(&mut self.live, set);
        self.retire(outgoing);
        self.overloaded = false;
        self.cost = None;
        self.estimate = None;
    }

    /// Creates a `HotSwap` instance holding a single static Set without a worker thread.
    pub fn fixed(live: Set) -> HotSwap {
        let (_, done_rx) = mpsc::channel();
        let live_viewport = live.viewport();
        HotSwap {
            live,
            cost: None,
            estimate: None,
            budget_ms: f32::INFINITY,
            overloaded: false,
            viewport: (1, 1),
            period: Period::new(),
            frames: 0,
            events: Vec::with_capacity(EVENT_CAPACITY),
            done: done_rx,
            measure_at: Arc::new(AtomicU64::new(packed(live_viewport))),
            graveyard: Arc::new(Mutex::new(Vec::new())),
            retired: Vec::new(),
            stop: Arc::new(AtomicBool::new(false)),
            worker: None,
            worker_lost: false,
        }
    }

    /// Advances the frame boundary, processes pending worker builds, and returns the live Set.
    ///
    /// Evaluates incoming candidate Sets against the frame budget and flags the slot
    /// as overloaded if the candidate exceeds it.
    pub fn begin_frame(&mut self, device: &wgpu::Device) -> &mut Set {
        self.frame_boundary(device);
        &mut self.live
    }

    /// Advances the frame boundary and installs builds for a slot that is currently off-air.
    pub(crate) fn begin_frame_parked(&mut self, device: &wgpu::Device) {
        self.frame_boundary(device);
    }

    fn frame_boundary(&mut self, device: &wgpu::Device) {
        self.period.mark(Instant::now());
        self.frames += 1;
        self.hand_over_retired();
        self.install_if_ready(device);
    }

    /// Returns an immutable reference to the live Set.
    pub fn set(&self) -> &Set {
        &self.live
    }

    /// Returns a mutable reference to the live Set without advancing frame boundary logic.
    pub(crate) fn live_mut(&mut self) -> &mut Set {
        &mut self.live
    }

    /// Returns an immutable reference to the live Set.
    pub fn live(&self) -> &Set {
        &self.live
    }

    /// Frames begun since construction. What "a build does not block the
    /// render loop" is measured in: the count between a request going out and
    /// its [`Event::Swapped`] coming back is how many frames were produced
    /// while the worker was busy.
    pub fn frames_rendered(&self) -> u64 {
        self.frames
    }

    /// Returns the per-frame candidate budget in milliseconds.
    pub fn budget_ms(&self) -> f32 {
        self.budget_ms
    }

    /// Returns whether the active Set has been stopped due to exceeding the frame budget.
    pub fn overloaded(&self) -> bool {
        self.overloaded
    }

    /// Sets the frame budget in milliseconds against which candidates are evaluated.
    pub fn set_budget_ms(&mut self, budget_ms: f32) {
        if budget_ms.is_finite() && budget_ms > 0.0 {
            self.budget_ms = budget_ms;
        }
    }

    /// Returns the rolling median frame period in milliseconds, or `None` if window is not full.
    pub fn frame_period_ms(&self) -> Option<f32> {
        self.period.median_ms()
    }

    /// Returns the probe measurement of the live Set, if measured.
    pub fn measured_cost(&self) -> Option<Measurement> {
        self.cost
    }

    /// Measures the live Set offscreen and records the result.
    ///
    /// Must not be called inside an active frame. Restores initial Set state
    /// via [`Set::rewind`] after measurement.
    pub fn measure_live(
        &mut self,
        probe: &mut Probe,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Measurement {
        let at = self.measure_size();
        let cost = measure(probe, device, queue, &mut self.live, at);
        self.cost = Some(cost);
        cost
    }

    /// Assigns a pre-computed probe measurement to the live Set.
    pub fn set_measured_cost(&mut self, cost: Measurement) {
        self.cost = Some(cost);
    }

    /// Returns the estimated cost of the live Set at target resolution, if computed.
    pub fn estimated_cost(&self) -> Option<&Estimate> {
        self.estimate.as_ref()
    }

    /// Estimates the cost of the live Set at `target` resolution and caches the result.
    pub fn estimate_live(
        &mut self,
        probe: &mut Probe,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: (u32, u32),
    ) -> &Estimate {
        let e = estimate(probe, device, queue, &mut self.live, target);
        self.estimate.insert(e)
    }

    /// Assigns a pre-computed estimate to the live Set.
    pub fn set_estimated_cost(&mut self, estimate: Estimate) {
        self.estimate = Some(estimate);
    }

    /// Drains and returns all queued lifecycle events.
    pub fn events(&mut self) -> std::vec::Drain<'_, Event> {
        self.events.drain(..)
    }

    /// Returns a slice of pending lifecycle events without draining them.
    pub fn pending_events(&self) -> &[Event] {
        &self.events
    }

    /// Resizes the live Set viewport to `(width, height)`.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.viewport = (width, height);
        self.live.resize(device, width, height);
        self.estimate = None;
    }

    /// Sets the target resolution for future worker probe measurements.
    pub fn set_measure_size(&mut self, at: (u32, u32)) {
        if at.0 == 0 || at.1 == 0 {
            return;
        }
        self.measure_at.store(packed(at), Ordering::Relaxed);
    }

    /// Returns the resolution configured for probe measurements.
    pub fn measure_size(&self) -> (u32, u32) {
        unpacked(self.measure_at.load(Ordering::Relaxed))
    }

    /// Evaluates candidate cost against `budget_ms` and updates overloaded state.
    fn judge(&mut self, id: u64, label: Arc<str>) {
        let (basis, budgeted) =
            governor::budgeted(self.cost, self.estimate.as_ref().map(Estimated::from));
        let judged = budgeted.filter(|ms| ms.is_finite());
        let basis = match judged {
            Some(_) => basis,
            None => Basis::Unbudgetable,
        };
        match judged {
            Some(cost_ms) if cost_ms > self.budget_ms => {
                self.overloaded = true;
                self.events.push(Event::Overloaded {
                    id,
                    label,
                    cost_ms,
                    basis,
                    budget_ms: self.budget_ms,
                });
            }
            _ => {
                self.events.push(Event::Accepted {
                    id,
                    label,
                    cost_ms: judged,
                    basis,
                    budget_ms: self.budget_ms,
                });
            }
        }
    }

    /// Installs and evaluates the newest completed build from the worker channel, if available.
    fn install_if_ready(&mut self, device: &wgpu::Device) {
        let mut newest: Option<Built> = None;
        loop {
            match self.done.try_recv() {
                Ok(Done::Refused(refusal)) => self.events.push(Event::SourceRefused {
                    label: refusal.label.into(),
                    said: refusal.said,
                }),
                Ok(Done::Built(next)) => {
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
                candidate.carry_moved_from(&self.live);
                candidate.carry_bound_from(&self.live);
                let outgoing = std::mem::replace(&mut self.live, candidate);
                self.cost = built.cost;
                self.estimate = None;
                self.retire(outgoing);
                self.overloaded = false;
                self.events.push(Event::Swapped {
                    id: built.id,
                    label: Arc::clone(&built.label),
                });
                self.judge(built.id, built.label);
            }
        }
    }

    /// Queues a replaced Set to be dropped by the worker thread.
    fn retire(&mut self, set: Set) {
        self.retired.push(set);
        self.hand_over_retired();
    }

    /// Transfers retired Sets to the shared graveyard mutex without blocking.
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

/// Initial capacity for the retired Set handover queue.
const GRAVEYARD_CAPACITY: usize = 4;

/// Initial capacity for the lifecycle event accumulator.
const EVENT_CAPACITY: usize = 4;

/// Packs `(width, height)` dimensions into a single `u64` for atomic storage.
fn packed((width, height): (u32, u32)) -> u64 {
    (u64::from(width) << 32) | u64::from(height)
}

/// Unpacks atomic `u64` dimensions into `(width, height)`.
fn unpacked(at: u64) -> (u32, u32) {
    ((at >> 32) as u32, at as u32)
}

/// Measures one frame execution cost of `set` at target resolution `at` and restores initial state.
///
/// Submits GPU commands and waits for completion. Rewinds buffer and simulation state via
/// [`Set::rewind`] prior to returning.
pub fn measure(
    probe: &mut Probe,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    set: &mut Set,
    at: (u32, u32),
) -> Measurement {
    let capacity = set.capacity();
    let viewport = set.viewport();
    probe.resize(device, at);
    set.resize(device, at.0, at.1);
    set.prepare(queue, PROBE_STEPS, &Signals::default());
    let measurement = probe.run(device, queue, set, PROBE_STEPS, capacity);
    set.rewind(device, queue);
    set.resize(device, viewport.0, viewport.1);
    measurement
}

/// Main execution loop for the background build and compilation worker.
///
/// Polls `source`, compiles requested Sets, performs probe measurements, and deallocates
/// retired Sets received from the render thread.
fn run_worker(
    device: wgpu::Device,
    queue: wgpu::Queue,
    mut source: Box<dyn Source>,
    out: Sender<Done>,
    graveyard: Arc<Mutex<Vec<Set>>>,
    stop: Arc<AtomicBool>,
    measure_at: Arc<AtomicU64>,
) {
    let mut probe: Option<Probe> = None;

    while !stop.load(Ordering::Relaxed) {
        let condemned: Vec<Set> = match graveyard.lock() {
            Ok(mut held) => held.drain(..).collect(),
            Err(_) => Vec::new(),
        };
        drop(condemned);

        let request = match source.poll() {
            Some(Polled::Build(request)) => request,
            Some(Polled::Refused(refusal)) => {
                if out.send(Done::Refused(refusal)).is_err() {
                    break;
                }
                continue;
            }
            None => continue,
        };
        let id = request.id;
        let label: Arc<str> = request.label.into();

        let build = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Set::build_many(
                &device,
                &queue,
                &request.l1s.iter().map(|(p, c)| (p, *c)).collect::<Vec<_>>(),
                &request.l2s.iter().collect::<Vec<_>>(),
                &request.l3s.iter().collect::<Vec<_>>(),
                &request.fields.iter().collect::<Vec<_>>(),
                &request.l4s.iter().collect::<Vec<_>>(),
                request.layering,
                request.seed_salt,
                &request.salts,
                crate::set::Wiring {
                    l1s: &request.names.l1s,
                    l2s: &request.names.l2s,
                    l3s: &request.names.l3s,
                    l4s: &request.names.l4s,
                    fields: &request.names.fields,
                    edges: &request.edges,
                },
            )
            .map(|mut set| {
                set.aim_camera(request.camera);
                if let Some(at) = request.live {
                    if !set.select_renderer(at as usize) {
                        eprintln!(
                            "  this build has no renderer {at} to fold to — every renderer is live"
                        );
                    }
                }
                for write in &request.params {
                    match set.write_param(write) {
                        Ok(0) => eprintln!("  no parameter named `{}`, ignoring", write.key),
                        Ok(_) => {}
                        Err(refused) => eprintln!("  {refused}"),
                    }
                }
                for control in request.published {
                    let name = control.name.clone();
                    if let Err(e) = set.publish(control) {
                        eprintln!("  `{name}` is not published: {e}");
                    }
                }
                for binding in request.bindings {
                    let (layer, key) = (binding.layer, binding.key.clone());
                    let signal = binding.signal.clone();
                    match set.bind(binding) {
                        crate::set::Bound::Yes => {}
                        crate::set::Bound::NoSuchParam => {
                            eprintln!("  no {layer:?} parameter named `{key}` to bind, ignoring")
                        }
                        crate::set::Bound::NoSuchControl => eprintln!(
                            "  `{signal}` is not published by this Set, so `{layer:?} {key}` \
                             is not bound"
                        ),
                    }
                }
                for stated in &request.authorities {
                    let (layer, index) = stated.at;
                    if !set.set_authority(layer, index, stated.authority) {
                        eprintln!(
                            "  this build has no {layer:?} node {index} to give authority to, \
                             ignoring"
                        );
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
            // Flush initial element buffer uploads so the Set is resident on the GPU.
            queue.submit([]);
            let _ = device.poll(wgpu::PollType::wait_indefinitely());

            let at = unpacked(measure_at.load(Ordering::Relaxed));
            let probe = probe.get_or_insert_with(|| {
                Probe::new(
                    &device,
                    &queue,
                    device.features().contains(wgpu::Features::TIMESTAMP_QUERY),
                    at,
                )
            });
            cost = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                measure(probe, &device, &queue, set, at)
            }))
            .ok();
            if cost.is_none() {
                eprintln!("  `{label}` could not be measured; it will not be budgeted for");
            }

            // Flush rewind uploads so they do not stall the render thread upon installation.
            queue.submit([]);
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
        }

        if out
            .send(Done::Built(Built {
                id,
                label,
                result,
                cost,
            }))
            .is_err()
        {
            break;
        }
    }
}

/// Extracts a displayable message from a caught panic payload.
fn panic_detail(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "no message".to_string()
    }
}

// Static assertion ensuring Set and Request remain Send across thread boundaries.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<Set>();
    assert_send::<Request>();
};
