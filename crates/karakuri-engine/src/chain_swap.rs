//! Master chain compilation on a worker thread, installed atomically at frame boundaries.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use karakuri_ir::typed::Checked;

use crate::master::{Chain, ChainTargets, ChainWorkshop, RetiredChain, Slot, SlotError, SlotSpec};
use crate::present::Present;
use crate::swap::POLL_INTERVAL;

/// Initial capacity for the retired chain handover queue.
const GRAVEYARD_CAPACITY: usize = 2;

/// Initial capacity for the lifecycle event accumulator.
const EVENT_CAPACITY: usize = 2;

/// Slot specification and checked source representation for chain compilation.
pub struct ChainSlot {
    pub spec: SlotSpec,
    pub checked: Checked,
}

/// A chain to build off the render thread.
struct Job {
    id: u64,
    slots: Vec<ChainSlot>,
    workshop: ChainWorkshop,
}

/// Which slot of a requested chain refused, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainRefusal {
    /// Position of the refusing slot in the requested list.
    pub at: usize,
    pub error: SlotError,
}

impl std::fmt::Display for ChainRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "master chain slot {}: {}", self.at, self.error)
    }
}

/// Worker response: a finished chain, or the slot that refused.
struct Done {
    id: u64,
    result: Result<Chain, ChainRefusal>,
}

/// Events emitted across the chain swap lifecycle.
#[derive(Debug)]
pub enum ChainEvent {
    /// A build was installed at a frame boundary.
    Installed { id: u64, slots: usize },
    /// A build refused. The chain that is running is unchanged.
    Refused { id: u64, refusal: ChainRefusal },
    /// The worker thread terminated unexpectedly.
    WorkerLost,
}

impl std::fmt::Display for ChainEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainEvent::Installed { slots: 0, .. } => {
                write!(
                    f,
                    "the master chain is empty; the mix goes straight to the tone map"
                )
            }
            ChainEvent::Installed { slots, .. } => {
                write!(f, "the master chain is {slots} slots and is running")
            }
            ChainEvent::Refused { refusal, .. } => {
                write!(f, "{refusal} — the chain keeps what it had")
            }
            ChainEvent::WorkerLost => write!(
                f,
                "the chain worker is gone; nothing more will be built this run \
                 (the chain that is running is unaffected)"
            ),
        }
    }
}

/// Manages asynchronous master chain compilation and frame-boundary installation.
pub struct ChainSwap {
    /// Requests to the worker. `None` once the worker has gone.
    jobs: Option<Sender<Job>>,
    done: Receiver<Done>,
    graveyard: Arc<Mutex<Vec<RetiredChain>>>,
    /// Chains retired by the render thread pending handover to the worker.
    retired: Vec<RetiredChain>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    /// The newest build in flight: its id and the list it is of. `None` where
    /// nothing is in flight.
    building: Option<(u64, Vec<SlotSpec>)>,
    events: Vec<ChainEvent>,
    next_id: u64,
    /// Whether worker channel disconnection has already been reported.
    worker_lost: bool,
    /// Chains installed since construction.
    installs: u64,
    /// Builds the worker has finished, whether or not they were installed.
    built: Arc<AtomicU64>,
}

impl ChainSwap {
    /// Creates a coordinator backed by a `karakuri-chain` worker thread.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> ChainSwap {
        let (jobs_tx, jobs_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let graveyard = Arc::new(Mutex::new(Vec::with_capacity(GRAVEYARD_CAPACITY)));
        let stop = Arc::new(AtomicBool::new(false));

        let built = Arc::new(AtomicU64::new(0));

        let worker = {
            let device = device.clone();
            let queue = queue.clone();
            let graveyard = Arc::clone(&graveyard);
            let stop = Arc::clone(&stop);
            let built = Arc::clone(&built);
            std::thread::Builder::new()
                .name("karakuri-chain".into())
                .spawn(move || run_worker(device, queue, jobs_rx, done_tx, graveyard, stop, built))
                .expect("spawn chain worker")
        };

        ChainSwap {
            jobs: Some(jobs_tx),
            done: done_rx,
            graveyard,
            retired: Vec::with_capacity(GRAVEYARD_CAPACITY),
            stop,
            worker: Some(worker),
            building: None,
            events: Vec::with_capacity(EVENT_CAPACITY),
            next_id: 0,
            worker_lost: false,
            installs: 0,
            built,
        }
    }

    /// The list the newest build in flight is of, or `None` where nothing is
    /// being built.
    pub fn building(&self) -> Option<&[SlotSpec]> {
        self.building.as_ref().map(|(_, slots)| slots.as_slice())
    }

    /// Returns true if an in-flight build matches `slots`.
    pub fn is_building(&self, slots: &[SlotSpec]) -> bool {
        self.building().is_some_and(|want| want == slots)
    }

    /// Enqueues a chain compilation job and returns its tracking ID.
    pub fn request(&mut self, present: &Present, slots: Vec<ChainSlot>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let wanted: Vec<SlotSpec> = slots.iter().map(|s| s.spec.clone()).collect();
        let job = Job {
            id,
            slots,
            workshop: present.chain_workshop(),
        };
        let sent = self
            .jobs
            .as_ref()
            .is_some_and(|jobs| jobs.send(job).is_ok());
        if sent {
            self.building = Some((id, wanted));
        } else {
            self.jobs = None;
            self.lost();
        }
        id
    }

    /// Advances the frame boundary: delegates retired chains and installs newest finished build.
    pub fn begin_frame(
        &mut self,
        present: &mut Present,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.hand_over_retired();
        self.install_if_ready(present, device, queue);
    }

    /// Drains and returns all queued lifecycle events.
    pub fn events(&mut self) -> std::vec::Drain<'_, ChainEvent> {
        self.events.drain(..)
    }

    /// Returns pending lifecycle events without draining them.
    pub fn pending_events(&self) -> &[ChainEvent] {
        &self.events
    }

    /// Chains installed since construction. What "a chain build does not block
    /// the render loop" is counted against: the frames between a request and
    /// this moving are frames drawn while the worker was busy.
    pub fn installs(&self) -> u64 {
        self.installs
    }

    /// Builds the worker has finished, installed or not. Moves before
    /// [`ChainSwap::installs`] does, by however many frames pass between the
    /// build finishing and the next frame boundary — which is what says an
    /// install waits for a boundary rather than landing when the build does.
    pub fn built(&self) -> u64 {
        self.built.load(Ordering::Relaxed)
    }

    /// Installs the newest finished build and retires every other one.
    fn install_if_ready(
        &mut self,
        present: &mut Present,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let mut newest: Option<Done> = None;
        loop {
            match self.done.try_recv() {
                Ok(next) => {
                    if let Some(stale) = newest.replace(next) {
                        self.discard(stale);
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.lost();
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }
        let Some(done) = newest else {
            return;
        };
        if self.building.as_ref().is_some_and(|(id, _)| *id == done.id) {
            self.building = None;
        }
        match done.result {
            Err(refusal) => self.events.push(ChainEvent::Refused {
                id: done.id,
                refusal,
            }),
            Ok(chain) => {
                let slots = chain.len();
                let outgoing = present.set_chain(device, queue, chain);
                self.retire(outgoing);
                self.installs += 1;
                self.events
                    .push(ChainEvent::Installed { id: done.id, slots });
            }
        }
    }

    /// Drops a build that a newer one superseded, by way of the worker.
    fn discard(&mut self, done: Done) {
        match done.result {
            Ok(chain) => self.retire(RetiredChain::from(chain)),
            Err(refusal) => self.events.push(ChainEvent::Refused {
                id: done.id,
                refusal,
            }),
        }
    }

    /// Queues a displaced chain to be dropped by the worker thread.
    fn retire(&mut self, chain: RetiredChain) {
        if chain.is_empty() {
            return;
        }
        self.retired.push(chain);
        self.hand_over_retired();
    }

    /// Transfers retired chains to the shared graveyard without blocking.
    fn hand_over_retired(&mut self) {
        if self.retired.is_empty() {
            return;
        }
        if let Ok(mut graveyard) = self.graveyard.try_lock() {
            graveyard.append(&mut self.retired);
        }
    }

    /// Reports the worker's loss once, and never again.
    fn lost(&mut self) {
        if self.worker.is_some() && !self.worker_lost {
            self.worker_lost = true;
            self.building = None;
            self.events.push(ChainEvent::WorkerLost);
        }
    }
}

impl Drop for ChainSwap {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.jobs = None;
        if let Some(worker) = self.worker.take() {
            // Up to one `POLL_INTERVAL`, because the worker is most likely
            // sitting in `recv_timeout`.
            let _ = worker.join();
        }
    }
}

/// Main execution loop for the chain build worker.
///
/// Builds requested chains and drops the chains the render thread retired.
fn run_worker(
    device: wgpu::Device,
    queue: wgpu::Queue,
    jobs: Receiver<Job>,
    out: Sender<Done>,
    graveyard: Arc<Mutex<Vec<RetiredChain>>>,
    stop: Arc<AtomicBool>,
    built: Arc<AtomicU64>,
) {
    while !stop.load(Ordering::Relaxed) {
        let condemned: Vec<RetiredChain> = match graveyard.lock() {
            Ok(mut held) => held.drain(..).collect(),
            Err(_) => Vec::new(),
        };
        drop(condemned);

        let job = match jobs.recv_timeout(POLL_INTERVAL) {
            Ok(job) => job,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        let id = job.id;
        let result = build(&device, &job);

        // Work moved off the render thread is not off it until its side
        // effects are too (ADR-0033): whatever this build left staged on the
        // queue is flushed here rather than by the render thread's next
        // submit.
        queue.submit([]);
        let _ = device.poll(wgpu::PollType::wait_indefinitely());

        if out.send(Done { id, result }).is_err() {
            break;
        }
        built.fetch_add(1, Ordering::Release);
    }
}

/// Compiles every slot of a job and allocates the targets they run through.
fn build(device: &wgpu::Device, job: &Job) -> Result<Chain, ChainRefusal> {
    let mut slots = Vec::with_capacity(job.slots.len());
    for (at, want) in job.slots.iter().enumerate() {
        let slot = Slot::build(
            device,
            job.workshop.layout(),
            want.spec.procedure.clone(),
            &want.checked,
            want.spec.cut,
            want.spec.params.clone(),
        )
        .map_err(|error| ChainRefusal { at, error })?;
        slots.push(slot);
    }
    let targets = ChainTargets::build(device, &job.workshop, &slots);
    Ok(Chain::resident(slots, targets))
}

// Static assertion ensuring a chain, its slots, what is built and what is
// retired all remain Send: the build happens on `karakuri-chain` and the drop
// happens there too.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<Slot>();
    assert_send::<Chain>();
    assert_send::<ChainTargets>();
    assert_send::<ChainWorkshop>();
    assert_send::<ChainSlot>();
    assert_send::<RetiredChain>();
};
