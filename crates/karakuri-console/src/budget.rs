//! Declarations for panel region update budgets, costs, and refresh intervals.
//!
//! Defines [`Declared`] costs and staleness tolerances checked by schedulability tests
//! without runtime arbitration (ADR-0164, ADR-0210, ADR-0212, ADR-0283, ADR-0330).

use std::time::Duration;

/// A live region's update declaration: cost, allowed staleness, and next change.
///
/// Declarations address arrangement nodes rather than individual animations (ADR-0156,
/// ADR-0159, ADR-0190, ADR-0206, ADR-0210).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Declared {
    /// The arrangement's name for the region that is declaring — `"transport"` and
    /// `"mixer"` today, in that order, which is the order they are drawn down the
    /// panel.
    pub region: &'static str,
    /// What one update of this region costs, on the CPU. Today that is
    /// [`PANEL_PASS`] for every region, and the constant says why.
    pub cost: Duration,
    /// Maximum allowed staleness in wall time before the region must be updated (ADR-0212).
    pub staleness: Duration,
    /// Duration until this region's picture next changes.
    ///
    /// Invariant: `moves_in >= staleness`. See ADR-0283 for details.
    pub moves_in: Duration,
}

/// Estimated CPU cost for a full panel redraw pass in immediate mode (1.26 ms).
///
/// Because immediate mode repaints the whole panel, each region currently declares
/// this full pass cost rather than a per-region slice (ADR-0164, ADR-0188, ADR-0190, ADR-0210).
pub const PANEL_PASS: Duration = Duration::from_micros(1260);

/// Maximum time the panel may spend per frame (16.67 ms at 60 Hz).
///
/// Acts as the budget divisor in ADR-0164's second condition; does not expand if the frame lengthens.
pub const BUDGET: Duration = Duration::from_nanos(16_666_667);

/// Nominal duration of one display frame (16.67 ms for 60 Hz).
///
/// Divisor for ADR-0164's first schedulability condition: `Σ (cost / staleness) ≤ budget / frame interval`.
pub const FRAME_INTERVAL: Duration = Duration::from_nanos(16_666_667);

/// Maximum fraction of [`BUDGET`] any single region update may consume (0.25).
///
/// Enforces ADR-0164's condition: `max(cost) ≤ a small part of the budget`.
pub const SMALL_PART: f32 = 0.25;
