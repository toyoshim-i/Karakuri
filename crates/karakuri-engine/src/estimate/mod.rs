//! Execution cost estimation from small-scale probe draws.
//!
//! Models execution cost as `a + b * area`, separating target-invariant compute
//! and vertex work `a` from area-dependent fragment work `b * area` using two
//! probe draws at reduced resolutions.

use karakuri_ir::rate::{Bound, RateBound};
use karakuri_ir::Topology;

use crate::probe::{Measurement, MeasurementMethod, Probe};
use crate::set::Set;
use crate::swap::PROBE_STEPS;
use crate::Signals;

/// Reference upper rung resolution for a 1280x720 target.
pub const PREPARATION_RESOLUTION: (u32, u32) = (640, 360);

/// Returns the minimum target height in rows at which a primitive with the
/// given point rate is at least one pixel.
pub fn sub_pixel_floor_rows(rate: f32) -> Option<u32> {
    if !rate.is_finite() || rate <= 0.0 {
        return None;
    }
    let rows = (1.0 / f64::from(rate)).ceil();
    if rows > f64::from(u32::MAX) {
        return None;
    }
    Some((rows as u32).max(1))
}

/// Maximum allowed share of target fragment cost that sub-pixel flooring may
/// obscure (0.25).
pub const FLOORED_SHARE_ALLOWED: f64 = 0.25;

/// Calculates the upper probe rung height ensuring obscured fragment share does
/// not exceed `FLOORED_SHARE_ALLOWED`.
pub fn upper_rung_rows(rows: u32) -> u32 {
    let rows = rows.max(1);
    let wanted = f64::from(rows) * (1.0 - FLOORED_SHARE_ALLOWED).sqrt();
    (wanted.ceil() as u32).clamp(1, rows)
}

/// Computes the fraction of target fragment cost obscured by sub-pixel flooring
/// across two probe rungs.
pub fn floored_share(low: (u32, u32), high: (u32, u32), target: (u32, u32), floor: u32) -> f64 {
    let rows = f64::from(target.1.max(1));
    let p = rows / f64::from(low.1.max(1));
    let q = rows / f64::from(high.1.max(1));
    if q <= 1.0 {
        return 0.0;
    }
    let x = rows / f64::from(floor.max(1));
    if x >= p {
        return 0.0;
    }
    let spread = area(high) - area(low);
    let lambda = if spread > 0.0 {
        (area(target) - area(low)) / spread
    } else {
        f64::NAN
    };
    if !lambda.is_finite() || lambda <= 1.0 {
        return (1.0 - 1.0 / (q * q)).clamp(0.0, 1.0);
    }
    let falling = |x: f64| (lambda - 1.0) * (1.0 - x * x / (p * p)) / (x * x);
    if x > q {
        return falling(x).clamp(0.0, 1.0);
    }
    (1.0 - 1.0 / (q * q)).max(falling(q)).clamp(0.0, 1.0)
}

/// Computes the maximum sub-pixel floor in rows required across all rate
/// bounds.
pub fn floor_rows(bounds: &[RateBound]) -> Result<u32, Unfit> {
    let mut floor = 1;
    for bound in bounds {
        match bound.bound {
            Bound::NoPrimitive => {}
            Bound::AtLeast { rate, .. } => match sub_pixel_floor_rows(rate) {
                Some(rows) => floor = floor.max(rows),
                None => return Err(Unfit::FloorUnknown),
            },
            Bound::Unbounded { .. } => return Err(Unfit::FloorUnknown),
        }
    }
    Ok(floor)
}

/// Provenance of the sub-pixel floor used for probe placement.
#[derive(Debug, Clone, PartialEq)]
pub enum Floor {
    /// Explicitly provided by the caller.
    Stated,
    /// Derived from static analysis of procedure rate expressions.
    Analysed {
        /// Analyzed rate bounds for each procedure in slot order.
        bounds: Vec<RateBound>,
        /// Parameter values violating declared bounds, if detected.
        contradicted: Option<(String, f32, [f32; 2])>,
    },
}

/// Sub-pixel flooring correction details.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Floored {
    /// Estimated share of fragment work obscured by flooring.
    pub share: f64,
    /// Scaling multiplier applied to the final estimate `(1 / (1 - share))`.
    pub correction: f64,
}

/// Calculates low and high probe resolutions for a given target and sub-pixel
/// floor.
pub fn rungs(target: (u32, u32), floor: u32) -> Result<[(u32, u32); 2], Unfit> {
    let rows = target.1.max(1);
    let floor = floor.max(1);
    let high = (rows / 2).max(1);
    let low = (rows / 4).max(1).max(floor);
    if high >= floor && low < high {
        return Ok([at_rows(target, low), at_rows(target, high)]);
    }
    let low = (rows / 4).max(1);
    let placed = at_rows(target, low);
    for high in upper_rung_rows(rows)..rows {
        if high <= low {
            continue;
        }
        let up = at_rows(target, high);
        if floored_share(placed, up, target, u32::MAX) <= FLOORED_SHARE_ALLOWED {
            return Ok([placed, up]);
        }
    }
    Err(Unfit::NoRoomBelowTheTarget { floor, target })
}

/// Scales target resolution to the given height preserving aspect ratio.
fn at_rows(target: (u32, u32), rows: u32) -> (u32, u32) {
    let height = u64::from(target.1.max(1));
    let width = (u64::from(target.0.max(1)) * u64::from(rows) / height).max(1);
    (width.min(u64::from(u32::MAX)) as u32, rows.max(1))
}

/// Computes pixel area for a resolution.
fn area((w, h): (u32, u32)) -> f64 {
    f64::from(w.max(1)) * f64::from(h.max(1))
}

/// Reference `ops_per_fragment` for the shipped master chain procedures.
pub const CHAIN_REFERENCE_OPS: u32 = 3947;

/// Target resolution used to establish [`CHAIN_REFERENCE_MS`] (ADR-0303).
pub const CHAIN_REFERENCE_SIZE: (u32, u32) = (1280, 720);

/// Reference execution time in milliseconds for [`CHAIN_REFERENCE_OPS`] at [`CHAIN_REFERENCE_SIZE`].
pub const CHAIN_REFERENCE_MS: f32 = 0.98;

/// Computes master chain execution cost in milliseconds for `ops_per_fragment` at `target` (ADR-0340).
pub fn chain_ms(ops_per_fragment: u32, target: (u32, u32)) -> f32 {
    if ops_per_fragment == 0 {
        return 0.0;
    }
    let rate = f64::from(CHAIN_REFERENCE_MS)
        / (f64::from(CHAIN_REFERENCE_OPS) * area(CHAIN_REFERENCE_SIZE));
    let ms = rate * f64::from(ops_per_fragment) * area(target);
    if ms.is_finite() {
        ms as f32
    } else {
        0.0
    }
}

/// Execution cost estimate for a Set at a target resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct Estimate {
    /// Target resolution of the estimate.
    pub target: (u32, u32),
    /// Drawn topologies in renderer order.
    pub topologies: Vec<Topology>,
    /// Sub-pixel floor in rows, if established.
    pub floor: Option<u32>,
    /// Source and method used to establish the sub-pixel floor.
    pub floor_from: Floor,
    /// Sub-pixel flooring correction details, if applied.
    pub floored: Option<Floored>,
    /// Measured probe samples for the low and high rungs.
    pub rungs: Option<[Measurement; 2]>,
    /// Linear fit result, or failure reason.
    pub fit: Result<Fit, Unfit>,
}

impl Estimate {
    /// Returns estimated execution time in milliseconds at target resolution, if
    /// successfully fitted.
    pub fn ms(&self) -> Option<f32> {
        self.fit.as_ref().ok().map(|f| f.ms)
    }

    /// Returns the measurement clock method used by the probe rungs.
    pub fn method(&self) -> Option<MeasurementMethod> {
        self.rungs.as_ref().map(|[low, _]| low.method)
    }

    /// Returns true if the estimate was derived from host clock measurements.
    pub fn biased_high(&self) -> bool {
        self.method() == Some(MeasurementMethod::HostWallClock)
    }

    /// Returns this estimate re-evaluated at `target` without re-running probe draws.
    pub fn at(&self, target: (u32, u32)) -> Estimate {
        let Some([low, high]) = self.rungs else {
            return Estimate {
                target,
                ..self.clone()
            };
        };
        let mut refitted = fit(
            low,
            high,
            target,
            self.floor.unwrap_or(u32::MAX),
            self.topologies.clone(),
        );
        refitted.floor = self.floor;
        refitted.floor_from = self.floor_from.clone();
        refitted
    }
}

/// Parameters of the linear cost model `a + b * area`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    /// Estimated frame time in milliseconds at target resolution.
    pub ms: f32,
    /// Invariant term `a` in milliseconds (compute, vertex, and submission
    /// overhead).
    pub invariant_ms: f32,
    /// Area-dependent fragment term `b * area` in milliseconds.
    pub fragment_ms: f32,
    /// Model slope `b` in milliseconds per pixel.
    pub ms_per_pixel: f64,
}

/// Reason why linear cost estimation failed or could not be performed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unfit {
    /// Sub-pixel floor could not be bounded away from zero.
    FloorUnknown,
    /// Target resolution is too small to accommodate distinct probe rungs below the
    /// floor.
    NoRoomBelowTheTarget {
        /// Sub-pixel floor in rows.
        floor: u32,
        /// Target resolution.
        target: (u32, u32),
    },
    /// Obscured fragment share exceeds allowed threshold.
    FlooringHidesTooMuch {
        /// Measured obscured share.
        share: f64,
        /// Maximum allowed share.
        allowed: f64,
    },
    /// Low and high probe rungs have identical resolution.
    RungsCoincide {
        /// Area of the identical rungs.
        area: f64,
    },
    /// Computed slope `b` is negative.
    FragmentTermNegative {
        /// Computed negative slope in milliseconds per pixel.
        ms_per_pixel: f64,
    },
    /// Computed invariant term `a` is negative.
    InvariantTermNegative {
        /// Computed negative invariant term in milliseconds.
        invariant_ms: f32,
    },
    /// Probe rungs used different measurement clock methods.
    InstrumentsDiffer,
    /// Numerical calculation produced NaN or infinity.
    NotFinite,
}

/// Fits `a + b * area` from two measurements and evaluates at target
/// resolution.
pub fn fit(
    first: Measurement,
    second: Measurement,
    target: (u32, u32),
    floor: u32,
    topologies: Vec<Topology>,
) -> Estimate {
    let [low, high] = if area(first.resolution) <= area(second.resolution) {
        [first, second]
    } else {
        [second, first]
    };
    let share = floored_share(low.resolution, high.resolution, target, floor);
    let floored = (share > 0.0).then(|| Floored {
        share,
        correction: 1.0 / (1.0 - share),
    });
    let refuse = |why: Unfit| Estimate {
        target,
        topologies: topologies.clone(),
        floor: Some(floor),
        floor_from: Floor::Stated,
        floored,
        rungs: Some([low, high]),
        fit: Err(why),
    };

    if low.method != high.method {
        return refuse(Unfit::InstrumentsDiffer);
    }
    if share > FLOORED_SHARE_ALLOWED {
        return refuse(Unfit::FlooringHidesTooMuch {
            share,
            allowed: FLOORED_SHARE_ALLOWED,
        });
    }
    let (a_low, a_high) = (area(low.resolution), area(high.resolution));
    if a_low == a_high {
        return refuse(Unfit::RungsCoincide { area: a_low });
    }

    let b = (f64::from(high.ms) - f64::from(low.ms)) / (a_high - a_low);
    let a = f64::from(low.ms) - b * a_low;
    if !b.is_finite() || !a.is_finite() {
        return refuse(Unfit::NotFinite);
    }
    if b < 0.0 {
        return refuse(Unfit::FragmentTermNegative { ms_per_pixel: b });
    }
    if a < 0.0 {
        return refuse(Unfit::InvariantTermNegative {
            invariant_ms: a as f32,
        });
    }
    let fragment = b * area(target);
    let ms = (a + fragment) / (1.0 - share);
    if !ms.is_finite() {
        return refuse(Unfit::NotFinite);
    }
    Estimate {
        target,
        topologies,
        floor: Some(floor),
        floor_from: Floor::Stated,
        floored,
        rungs: Some([low, high]),
        fit: Ok(Fit {
            ms: ms as f32,
            invariant_ms: a as f32,
            fragment_ms: fragment as f32,
            ms_per_pixel: b,
        }),
    }
}

/// Estimates Set execution time at target resolution by probing with analyzed
/// sub-pixel floors.
pub fn estimate(
    probe: &mut Probe,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    set: &mut Set,
    target: (u32, u32),
) -> Estimate {
    let from = Floor::Analysed {
        bounds: set.rate_bounds().to_vec(),
        contradicted: set.rate_bound_contradicted(),
    };
    let Floor::Analysed {
        bounds,
        contradicted,
    } = &from
    else {
        unreachable!("just built as Analysed")
    };
    let known = if contradicted.is_some() {
        None
    } else {
        floor_rows(bounds).ok()
    };
    let mut e = estimate_above_floor(probe, device, queue, set, target, known.unwrap_or(u32::MAX));
    e.floor = known;
    e.floor_from = from;
    e
}

/// Estimates Set execution time at target resolution given an explicit
/// sub-pixel floor.
pub fn estimate_above_floor(
    probe: &mut Probe,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    set: &mut Set,
    target: (u32, u32),
    floor: u32,
) -> Estimate {
    let topologies = set.drawn_topologies();
    let placed = match rungs(target, floor) {
        Ok(placed) => placed,
        Err(why) => {
            return Estimate {
                target,
                topologies,
                floor: Some(floor),
                floor_from: Floor::Stated,
                floored: None,
                rungs: None,
                fit: Err(why),
            }
        }
    };

    let capacity = set.capacity();
    let viewport = set.viewport();
    let mut taken = Vec::with_capacity(2);
    for size in placed {
        probe.resize(device, size);
        set.resize(device, size.0, size.1);
        set.prepare(queue, PROBE_STEPS, &Signals::default());
        taken.push(probe.run(device, queue, set, PROBE_STEPS, capacity));
        set.rewind(device, queue);
    }
    set.resize(device, viewport.0, viewport.1);

    fit(taken[0], taken[1], target, floor, topologies)
}

#[cfg(test)]
mod tests;
