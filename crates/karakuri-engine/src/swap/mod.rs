//! Hot swap coordination and watchdog budgeting (background compilation and atomic installation).

mod types;
mod worker;

pub use types::*;
pub use worker::*;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use crate::estimate::{estimate, Estimate};
use crate::governor::{self, Basis, Estimated};
use crate::probe::{Measurement, Probe};
use crate::set::Set;

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
    /// Output resolution the worker's `estimate` answers for, packed as `(width << 32) | height`.
    estimate_at: Arc<AtomicU64>,
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
        // The viewport below, and not the Set's: nothing has told this slot
        // what it draws into yet, so the worker's `estimate` answers for the
        // same 1x1 frame `install_if_ready` will re-target it to. A target
        // that small holds no rungs, so it refuses without spending a draw.
        let estimate_at = Arc::new(AtomicU64::new(packed((1, 1))));
        let sizes = Sizes {
            measure_at: Arc::clone(&measure_at),
            estimate_at: Arc::clone(&estimate_at),
        };

        let worker = {
            let device = device.clone();
            let queue = queue.clone();
            let graveyard = Arc::clone(&graveyard);
            let stop = Arc::clone(&stop);
            std::thread::Builder::new()
                .name("karakuri-build".into())
                .spawn(move || run_worker(device, queue, source, done_tx, graveyard, stop, sizes))
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
            estimate_at,
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
            // No worker, so nothing reads this; `resize` writes it anyway so a
            // fixed slot's estimate target is not a second rule.
            estimate_at: Arc::new(AtomicU64::new(packed((1, 1)))),
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

    /// Resizes the live Set viewport to `(width, height)` and re-targets the estimate arithmetic.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.viewport = (width, height);
        self.live.resize(device, width, height);
        self.estimate_at
            .store(packed((width, height)), Ordering::Relaxed);
        self.estimate = self.estimate.take().map(|e| e.at((width, height)));
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

    /// Returns the output resolution the worker's `estimate` answers for.
    pub fn estimate_size(&self) -> (u32, u32) {
        unpacked(self.estimate_at.load(Ordering::Relaxed))
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
                // Re-target candidate estimate to the active viewport (ADR-0356).
                self.estimate = built.estimate.map(|e| e.at(self.viewport));
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

// Static assertion ensuring Set and Request remain Send across thread boundaries.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<Set>();
    assert_send::<Request>();
};
