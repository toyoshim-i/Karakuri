//! Compute budget governor and admission controller for deck slot priming.
//!
//! Evaluates requested residency states against available GPU compute headroom,
//! admitting candidates to `Residency::Priming` or parking them as `Residency::Allocated`
//! when headroom is insufficient.
//!
//! Live slots are never modified or demoted by the governor. Budgets are evaluated
//! against slot estimates or probe measurements taken at set build time.

use crate::deck::Residency;
use crate::estimate::{Estimate, Floor, Floored, Unfit};
use crate::probe::{Measurement, MeasurementMethod};

/// Default compute budget in milliseconds (16.7 ms, corresponding to a 60 Hz frame interval).
pub const DEFAULT_COMPUTE_BUDGET_MS: f32 = 16.7;

/// Origin of the sub-pixel floor used during slot cost estimation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FloorRead {
    /// Floor was explicitly stated by the caller without static analysis.
    Stated,
    /// Floor was derived via static analysis of point_rate expressions.
    Analysed,
    /// Static analysis bound was contradicted by a runtime parameter value.
    Contradicted,
}

/// Cost estimation results and parameters for budget calculation.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Estimated {
    /// Estimated frame duration in milliseconds, or the reason estimation failed.
    pub fit: Result<f32, Unfit>,
    /// Target resolution for which the estimate was evaluated.
    pub target: (u32, u32),
    /// Measurement method used during estimation rungs.
    pub method: Option<MeasurementMethod>,
    /// Sub-pixel floor height in rows, if established.
    pub floor: Option<u32>,
    /// Derivation source of the sub-pixel floor.
    pub floor_from: FloorRead,
    /// Applied correction factor if estimation rungs fell below the sub-pixel floor.
    pub floored: Option<Floored>,
}

impl Estimated {
    /// Returns the estimated milliseconds if the fit succeeded.
    pub fn ms(&self) -> Option<f32> {
        self.fit.ok()
    }

    /// Returns whether the estimate includes a sub-pixel floor correction.
    pub fn corrected(&self) -> bool {
        self.floored.is_some()
    }
}

impl From<&Estimate> for Estimated {
    fn from(e: &Estimate) -> Estimated {
        Estimated {
            fit: e.fit.as_ref().map(|f| f.ms).map_err(|u| *u),
            target: e.target,
            method: e.method(),
            floor: e.floor,
            floor_from: match &e.floor_from {
                Floor::Stated => FloorRead::Stated,
                Floor::Analysed {
                    contradicted: Some(_),
                    ..
                } => FloorRead::Contradicted,
                Floor::Analysed { .. } => FloorRead::Analysed,
            },
            floored: e.floored,
        }
    }
}

/// Cost metric basis used for budget arithmetic.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Basis {
    /// Cost is unknown; neither measurement nor estimate is available.
    Unbudgetable,
    /// Single-draw measurement at reference resolution.
    Measured,
    /// Multi-draw polynomial fit evaluated at output resolution.
    Estimated,
}

/// Explanatory reason for a slot's effective residency assignment.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    /// Slot is currently Live and retained on air.
    OnAir,
    /// Slot requested Priming and fits within available compute headroom.
    Fits,
    /// Slot requested Allocated and remains off air.
    OffAir,
    /// Priming postponed: insufficient headroom remaining after Live slots.
    NoHeadroom,
    /// Priming postponed: slot cost is unmeasured and unestimated.
    Unmeasured,
    /// Priming postponed: committed cost of Live slots cannot be determined.
    CommittedUnknown,
    /// Priming skipped: closed-form procedure requires no state warm-up.
    NoPrimingNeeded,
}

/// Governor residency decision and cost basis for one slot.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Decision {
    pub slot: usize,
    /// Requested residency requested by the operator.
    pub requested: Residency,
    /// Effective residency assigned by the governor for this frame.
    pub effective: Residency,
    pub reason: Reason,
    /// Single-draw measured cost at reference resolution, if available.
    pub cost_ms: Option<f32>,
    /// Cost in milliseconds used for headroom arithmetic.
    pub budgeted_ms: Option<f32>,
    /// Source metric for `budgeted_ms`.
    pub basis: Basis,
    /// Cost estimate details, if available.
    pub estimate: Option<Estimated>,
}

impl Decision {
    /// Returns `true` if the slot requested `Priming` but was assigned `Allocated`.
    pub fn is_parked(self) -> bool {
        self.requested == Residency::Priming && self.effective == Residency::Allocated
    }
}

/// Result of a governor evaluation pass across all slots in a deck.
#[derive(Clone, Debug)]
pub struct Report {
    /// Every slot, in index order.
    pub decisions: Vec<Decision>,
    /// The budget these decisions were made against.
    pub budget_ms: f32,
    /// Summed budgeted compute time of active Live slots in milliseconds.
    pub committed_ms: f32,
    /// Summed budgeted compute time of admitted Priming slots in milliseconds.
    pub priming_ms: f32,
    /// Count of Live slots lacking both a measurement and an estimate.
    pub unmeasured_live: usize,
    /// True if the summed Live slot budget exceeds the compute budget.
    pub over_budget: bool,
    /// True if any metric in this report was derived from a host clock fallback.
    pub host_clock: bool,
    /// Measured rolling median frame period of the deck in milliseconds.
    pub frame_period_ms: Option<f32>,
    /// Target frame interval budget for the deck in milliseconds.
    pub frame_budget_ms: Option<f32>,
}

impl Report {
    /// Returns remaining compute budget headroom in milliseconds, or `None` if unmeasured Live slots exist.
    pub fn headroom_ms(&self) -> Option<f32> {
        self.committed_known()
            .then_some(self.budget_ms - self.committed_ms)
    }

    /// Returns true if all Live slots have known measurements or estimates.
    pub fn committed_known(&self) -> bool {
        self.unmeasured_live == 0
    }

    /// Returns an iterator over slots whose Priming request was deferred to Allocated.
    pub fn parked(&self) -> impl Iterator<Item = &Decision> {
        self.decisions.iter().filter(|d| d.is_parked())
    }

    /// Returns the number of slots budgeted using an area-scaled estimate.
    pub fn estimated(&self) -> usize {
        self.decisions
            .iter()
            .filter(|d| d.basis == Basis::Estimated)
            .count()
    }

    /// Returns the number of estimates that include a sub-pixel floor correction.
    pub fn corrected(&self) -> usize {
        self.decisions
            .iter()
            .filter(|d| d.basis == Basis::Estimated && d.estimate.is_some_and(|e| e.corrected()))
            .count()
    }

    /// Returns the number of estimates evaluated against an unknown floor.
    pub fn floor_unknown(&self) -> usize {
        self.decisions
            .iter()
            .filter(|d| {
                d.basis == Basis::Estimated && d.estimate.is_some_and(|e| e.floor.is_none())
            })
            .count()
    }

    /// Returns an iterator over slots whose estimate failed fit validation, along with the refusal cause.
    pub fn refused_estimates(&self) -> impl Iterator<Item = (usize, Unfit)> + '_ {
        self.decisions
            .iter()
            .filter_map(|d| d.estimate.and_then(|e| e.fit.err()).map(|u| (d.slot, u)))
    }

    /// Returns true if the rolling median frame period exceeds the frame interval budget.
    pub fn deck_over_period(&self) -> Option<bool> {
        Some(self.frame_period_ms? > self.frame_budget_ms?)
    }

    /// Returns the common target resolution `(width, height)` of the estimates in this report, if any.
    pub fn estimated_target(&self) -> Option<(u32, u32)> {
        self.decisions
            .iter()
            .filter(|d| d.basis == Basis::Estimated)
            .find_map(|d| d.estimate.map(|e| e.target))
    }
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "governor: {:.2} / {:.2} ms live",
            self.committed_ms, self.budget_ms
        )?;
        if self.priming_ms > 0.0 {
            write!(f, " + {:.2} priming", self.priming_ms)?;
        }
        if self.unmeasured_live > 0 {
            write!(f, " (+{} unmeasured)", self.unmeasured_live)?;
        }
        write!(
            f,
            " [{}]",
            if self.host_clock {
                "host clock"
            } else {
                "gpu timestamps"
            }
        )?;
        let mut said = false;
        if let Some((w, h)) = self.estimated_target() {
            write!(f, " — {} estimated at {w}x{h}", self.estimated())?;
            said = true;
            let corrected = self.corrected();
            if corrected > 0 {
                write!(f, ", {corrected} corrected for a floored rung")?;
            }
            let unknown = self.floor_unknown();
            if unknown > 0 {
                write!(f, ", {unknown} on an unknown floor")?;
            }
        }
        let refused = self.refused_estimates().count();
        if refused > 0 {
            write!(
                f,
                "{}{refused} refused, budgeted on the measurement",
                if said { ", " } else { " — " }
            )?;
        }
        if self.over_budget {
            write!(
                f,
                " — OVER: live slots exceed the budget, priming suspended"
            )?;
        } else if !self.committed_known() {
            write!(
                f,
                " — UNKNOWN: the committed cost cannot be known while a live slot \
                 is unmeasured, priming suspended"
            )?;
        }
        if let (Some(period), Some(budget)) = (self.frame_period_ms, self.frame_budget_ms) {
            write!(f, " — the deck's frames: {period:.2} / {budget:.2} ms")?;
            if period > budget {
                write!(f, ", OVER its period")?;
            }
        }
        Ok(())
    }
}

/// Slot state input for governor evaluation.
#[derive(Clone, Copy, Debug)]
pub struct SlotState {
    /// Operator-requested residency for this slot.
    pub requested: Residency,
    /// Reference-resolution GPU measurement for the slot, if available.
    pub cost: Option<Measurement>,
    /// Area-scaled execution cost estimate for the slot, if available.
    pub estimate: Option<Estimated>,
    /// Whether the slot procedure can evaluate arbitrary time offsets directly.
    pub closed_form: bool,
}

impl SlotState {
    /// Returns the governing basis and budgeted milliseconds for this slot.
    pub fn budgeted(&self) -> (Basis, Option<f32>) {
        budgeted(self.cost, self.estimate)
    }
}

/// Determines the budgeting basis and expected execution time for a slot.
///
/// Prefers an available cost estimate. Falls back to a direct measurement if the
/// estimate is unavailable or refused, and returns `(Basis::Unbudgetable, None)` if neither
/// is present.
pub fn budgeted(cost: Option<Measurement>, estimate: Option<Estimated>) -> (Basis, Option<f32>) {
    if let Some(ms) = estimate.and_then(|e| e.ms()) {
        return (Basis::Estimated, Some(ms));
    }
    match cost {
        Some(cost) => (Basis::Measured, Some(cost.ms)),
        None => (Basis::Unbudgetable, None),
    }
}

/// Evaluates slot residency requests against a target frame compute budget.
#[derive(Clone, Copy, Debug)]
pub struct Governor {
    budget_ms: f32,
}

impl Default for Governor {
    fn default() -> Governor {
        Governor::new(DEFAULT_COMPUTE_BUDGET_MS)
    }
}

impl Governor {
    pub fn new(budget_ms: f32) -> Governor {
        Governor { budget_ms }
    }

    pub fn budget_ms(&self) -> f32 {
        self.budget_ms
    }

    pub fn set_budget_ms(&mut self, budget_ms: f32) {
        self.budget_ms = budget_ms;
    }

    /// Evaluates slot residency requests against the compute budget and produces a Report.
    ///
    /// Iterates over slots in index order, reserving budget for Live slots first before
    /// evaluating Priming requests against remaining headroom.
    pub fn decide(&self, slots: &[SlotState]) -> Report {
        let mut committed_ms = 0.0f32;
        let mut unmeasured_live = 0usize;
        let mut host_clock = false;
        for slot in slots {
            if let Some(cost) = slot.cost {
                if cost.method == MeasurementMethod::HostWallClock {
                    host_clock = true;
                }
            }
            if slot.estimate.and_then(|e| e.method) == Some(MeasurementMethod::HostWallClock) {
                host_clock = true;
            }
            if slot.requested == Residency::Live {
                match slot.budgeted().1 {
                    Some(ms) => committed_ms += ms,
                    None => unmeasured_live += 1,
                }
            }
        }

        let over_budget = committed_ms > self.budget_ms;
        let committed_known = unmeasured_live == 0;
        let mut headroom = (self.budget_ms - committed_ms).max(0.0);
        let mut priming_ms = 0.0f32;

        let decisions = slots
            .iter()
            .enumerate()
            .map(|(i, slot)| {
                let cost_ms = slot.cost.map(|c| c.ms);
                let (basis, budgeted_ms) = slot.budgeted();
                let (effective, reason) = match slot.requested {
                    Residency::Live => (Residency::Live, Reason::OnAir),
                    Residency::Allocated => (Residency::Allocated, Reason::OffAir),
                    Residency::Priming => {
                        let (r, why) = self.admit(slot, &mut headroom, committed_known);
                        if r == Residency::Priming {
                            priming_ms += budgeted_ms.unwrap_or(0.0);
                        }
                        (r, why)
                    }
                };
                Decision {
                    slot: i,
                    requested: slot.requested,
                    effective,
                    reason,
                    cost_ms,
                    budgeted_ms,
                    basis,
                    estimate: slot.estimate,
                }
            })
            .collect();

        Report {
            decisions,
            budget_ms: self.budget_ms,
            committed_ms,
            priming_ms,
            unmeasured_live,
            over_budget,
            host_clock,
            frame_period_ms: None,
            frame_budget_ms: None,
        }
    }

    /// Evaluates whether a slot requesting Priming residency fits within available headroom.
    fn admit(
        &self,
        slot: &SlotState,
        headroom: &mut f32,
        committed_known: bool,
    ) -> (Residency, Reason) {
        if slot.closed_form {
            return (Residency::Allocated, Reason::NoPrimingNeeded);
        }
        let (_, budgeted) = slot.budgeted();
        let Some(ms) = budgeted else {
            return (Residency::Allocated, Reason::Unmeasured);
        };
        if !committed_known {
            return (Residency::Allocated, Reason::CommittedUnknown);
        }
        if ms.is_nan() || ms > *headroom {
            return (Residency::Allocated, Reason::NoHeadroom);
        }
        *headroom -= ms;
        (Residency::Priming, Reason::Fits)
    }
}
