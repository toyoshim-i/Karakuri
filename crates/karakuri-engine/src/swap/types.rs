use std::sync::atomic::AtomicU64;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;

use crate::binding::Binding;
use crate::estimate::Estimate;
use crate::governor::Basis;
use crate::probe::Measurement;
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

/// Initial capacity for the retired Set handover queue.
pub(crate) const GRAVEYARD_CAPACITY: usize = 4;

/// Initial capacity for the lifecycle event accumulator.
pub(crate) const EVENT_CAPACITY: usize = 4;

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
pub fn said(basis: Basis) -> &'static str {
    match basis {
        Basis::Measured => "one draw at the size the caller named, host clock",
        Basis::Estimated => "a two-draw fit at the output's size",
        Basis::Unbudgetable => "no number",
    }
}

/// Measurement and estimation resolutions shared between the worker and render threads.
#[derive(Clone)]
pub(crate) struct Sizes {
    pub(crate) measure_at: Arc<AtomicU64>,
    pub(crate) estimate_at: Arc<AtomicU64>,
}

/// Worker response containing either a finished build or a pre-build refusal.
#[allow(clippy::large_enum_variant)]
pub(crate) enum Done {
    Built(Built),
    Refused(Refusal),
}

/// Finished build result ready for installation on the render thread.
pub(crate) struct Built {
    pub(crate) id: u64,
    pub(crate) label: Arc<str>,
    pub(crate) result: Result<Set, SetError>,
    pub(crate) cost: Option<Measurement>,
    /// The worker's `estimate` of the built Set at the output's size, or its
    /// named refusal. `None` only where the estimate itself panicked; a
    /// refusal is `Some` carrying `Err(Unfit)`, because an estimate that
    /// declined to answer has not said the answer is small.
    pub(crate) estimate: Option<Estimate>,
}

/// Rolling median frame period measured across successive frame boundaries.
pub(crate) struct Period {
    samples: [f32; PERIOD_FRAMES],
    next: usize,
    filled: bool,
    last: Option<Instant>,
}

impl Period {
    pub(crate) fn new() -> Period {
        Period {
            samples: [0.0; PERIOD_FRAMES],
            next: 0,
            filled: false,
            last: None,
        }
    }

    /// Records a frame boundary timestamp and updates the rolling window.
    pub(crate) fn mark(&mut self, now: Instant) {
        if let Some(last) = self.last.replace(now) {
            self.samples[self.next] = now.duration_since(last).as_secs_f32() * 1_000.0;
            self.next = (self.next + 1) % PERIOD_FRAMES;
            self.filled |= self.next == 0;
        }
    }

    /// Returns the rolling median frame period in milliseconds, or `None` if window is not full.
    pub(crate) fn median_ms(&self) -> Option<f32> {
        if !self.filled {
            return None;
        }
        let mut sorted = self.samples;
        sorted.sort_by(|a, b| a.partial_cmp(b).expect("frame intervals are finite"));
        Some(sorted[PERIOD_FRAMES / 2])
    }
}

/// Packs `(width, height)` dimensions into a single `u64` for atomic storage.
pub(crate) fn packed((width, height): (u32, u32)) -> u64 {
    (u64::from(width) << 32) | u64::from(height)
}

/// Unpacks atomic `u64` dimensions into `(width, height)`.
pub(crate) fn unpacked(at: u64) -> (u32, u32) {
    ((at >> 32) as u32, at as u32)
}
