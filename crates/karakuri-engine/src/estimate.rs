//! Execution cost estimation from small-scale probe draws.
//!
//! Models execution cost as `a + b * area`, separating target-invariant compute and vertex
//! work `a` from area-dependent fragment work `b * area` using two probe draws at reduced
//! resolutions.

use karakuri_ir::rate::{Bound, RateBound};
use karakuri_ir::Topology;

use crate::probe::{Measurement, MeasurementMethod, Probe};
use crate::set::Set;
use crate::swap::PROBE_STEPS;
use crate::Signals;

/// Reference upper rung resolution for a 1280x720 target.
pub const PREPARATION_RESOLUTION: (u32, u32) = (640, 360);

/// Returns the minimum target height in rows at which a primitive with the given point rate is at least one pixel.
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

/// Maximum allowed share of target fragment cost that sub-pixel flooring may obscure (0.25).
pub const FLOORED_SHARE_ALLOWED: f64 = 0.25;

/// Calculates the upper probe rung height ensuring obscured fragment share does not exceed `FLOORED_SHARE_ALLOWED`.
pub fn upper_rung_rows(rows: u32) -> u32 {
    let rows = rows.max(1);
    let wanted = f64::from(rows) * (1.0 - FLOORED_SHARE_ALLOWED).sqrt();
    (wanted.ceil() as u32).clamp(1, rows)
}

/// Computes the fraction of target fragment cost obscured by sub-pixel flooring across two probe rungs.
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

/// Computes the maximum sub-pixel floor in rows required across all rate bounds.
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

/// Calculates low and high probe resolutions for a given target and sub-pixel floor.
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
    /// Returns estimated execution time in milliseconds at target resolution, if successfully fitted.
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
}

/// Parameters of the linear cost model `a + b * area`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    /// Estimated frame time in milliseconds at target resolution.
    pub ms: f32,
    /// Invariant term `a` in milliseconds (compute, vertex, and submission overhead).
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
    /// Target resolution is too small to accommodate distinct probe rungs below the floor.
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

/// Fits `a + b * area` from two measurements and evaluates at target resolution.
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

/// Estimates Set execution time at target resolution by probing with analyzed sub-pixel floors.
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

/// Estimates Set execution time at target resolution given an explicit sub-pixel floor.
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
mod tests {
    use super::*;

    /// **A target size to fit against**, and a fixture rather than a
    /// reference — see [`PREPARATION_RESOLUTION`]. It was
    /// `swap::AT` until ADR-0303 removed that constant.
    const AT: (u32, u32) = (1280, 720);

    fn measured(ms: f32, resolution: (u32, u32)) -> Measurement {
        Measurement {
            ms,
            method: MeasurementMethod::HostWallClock,
            capacity: 262_144,
            resolution,
        }
    }

    /// **The fit, against an input built from a known `a` and `b`.** Synthetic
    /// on purpose: the arithmetic is the whole of the rule, and a test of it
    /// against a real draw would be testing the machine.
    ///
    /// `a` is 9.0 ms and `b` is chosen so the fragment term is 9.2 ms at
    /// 1280x720 — the shape `soft_points` at `point_scale` 0.0222 showed, where
    /// half the frame moves with the target and half does not.
    #[test]
    fn two_rungs_recover_the_a_and_b_they_were_built_from() {
        let a = 9.0_f64;
        let b = 9.2 / area(AT);
        let (low, high) = ((320, 180), (640, 360));
        let e = fit(
            measured((a + b * area(low)) as f32, low),
            measured((a + b * area(high)) as f32, high),
            AT,
            180,
            vec![Topology::Points],
        );

        let f = e.fit.expect("two clean rungs fit");
        assert!(
            (f.invariant_ms - 9.0).abs() < 1e-2,
            "a came out {}",
            f.invariant_ms
        );
        assert!(
            (f.fragment_ms - 9.2).abs() < 1e-2,
            "b*A came out {}",
            f.fragment_ms
        );
        assert!((f.ms - 18.2).abs() < 2e-2, "the answer came out {}", f.ms);
        assert_eq!(e.floor, Some(180));
        assert_eq!(e.target, AT);
    }

    /// **The rungs may arrive in either order**, and the [`Estimate`] records
    /// them low area first whichever way they came.
    #[test]
    fn the_rungs_are_sorted_by_area_rather_than_trusted() {
        let (low, high) = ((320, 180), (640, 360));
        let e = fit(
            measured(4.0, high),
            measured(2.0, low),
            AT,
            180,
            vec![Topology::Fullscreen],
        );
        let [first, second] = e.rungs.expect("both rungs recorded");
        assert_eq!(first.resolution, low);
        assert_eq!(second.resolution, high);
        assert!(e.fit.is_ok());
    }

    /// **The whole point of the change, as a test.** The area-ratio rule turned
    /// `soft_points` — 8.996 ms at 640x360, 8.872 ms at 1280x720 — into an
    /// estimate of 36 ms, past the last of the console's five bands, which is
    /// the band that stops a slot. A second rung says the frame does not move
    /// with the target, so `b` is nearly nothing and the answer is nearly the
    /// measurement.
    ///
    /// The figures are the ones `examples/small_draw.rs` produced on one
    /// machine and are here as a *shape* rather than as a threshold: what is
    /// asserted is that a flat pair no longer multiplies.
    #[test]
    fn a_flat_pair_is_no_longer_multiplied_by_the_area_ratio() {
        let e = fit(
            measured(8.980, (320, 180)),
            measured(8.996, (640, 360)),
            AT,
            180,
            vec![Topology::Points],
        );
        let f = e.fit.expect("a flat pair is still a fit");
        assert!(
            f.ms < 12.0,
            "a frame that did not move with the target estimated at {:.2} ms, and the \
             area-ratio rule this replaced said 36",
            f.ms
        );
        assert!(
            f.fragment_ms < f.invariant_ms,
            "the fragment term is {:.2} ms against an invariant {:.2} ms, which is not \
             what a flat pair means",
            f.fragment_ms,
            f.invariant_ms
        );
    }

    /// **A negative `b` is a failed measurement, not a cheap Set.** Below
    /// 640x360 a quarter of a million primitives binned into too few tiles cost
    /// *more* on this crate's development machine, not less — 9.08 ms at
    /// 452x254 against 10.17 at 320x180 — so the smaller draw reads dearer and
    /// the slope comes out below zero. There is no such thing as a fragment
    /// stage that costs less than nothing.
    #[test]
    fn a_pair_that_got_dearer_as_it_got_smaller_is_refused() {
        let e = fit(
            measured(10.17, (320, 180)),
            measured(9.08, (452, 254)),
            AT,
            180,
            vec![Topology::Points],
        );
        match e.fit {
            Err(Unfit::FragmentTermNegative { ms_per_pixel }) => {
                assert!(ms_per_pixel < 0.0, "{ms_per_pixel} is not negative")
            }
            other => panic!("expected a negative fragment term, got {other:?}"),
        }
        assert_eq!(e.ms(), None, "a refusal hands out no number");
    }

    /// **A negative `a` is the same failure at the other end.** Two rungs whose
    /// line reaches zero area below zero milliseconds did not measure one
    /// curve: the simulation and the vertex stage cannot cost less than
    /// nothing.
    #[test]
    fn a_pair_whose_invariant_term_is_below_zero_is_refused() {
        // A slope steep enough that extrapolating back to zero area goes
        // negative: 1 ms at a sixteenth of the area and 8 ms at a quarter.
        let e = fit(
            measured(1.0, (320, 180)),
            measured(8.0, (640, 360)),
            AT,
            180,
            vec![Topology::Fullscreen],
        );
        match e.fit {
            Err(Unfit::InvariantTermNegative { invariant_ms }) => {
                assert!(invariant_ms < 0.0, "{invariant_ms} is not negative")
            }
            other => panic!("expected a negative invariant term, got {other:?}"),
        }
    }

    /// **A rung far enough under ADR-0245's floor is still refused.** The
    /// pair here is the *cheap* one — half and a quarter of the target's
    /// height — against `speed_lines`' 720-row floor, so the upper rung is at
    /// half the target and `1 - 1/q²` is 3/4: three quarters of the target's
    /// fragment cost could be hidden, against an allowance of a quarter.
    ///
    /// **This is what [`Unfit::RungBelowFloor`] used to be**, and the
    /// difference is the whole of ADR-0293: a rung under the floor is a
    /// question of how much, not a wall. [`rungs`] never produces this pair —
    /// it moves the upper rung — and a caller placing its own gets the number
    /// its placement earns.
    #[test]
    fn a_pair_that_would_hide_too_much_of_the_frame_is_refused() {
        let e = fit(
            measured(4.0, (320, 180)),
            measured(6.0, (640, 360)),
            AT,
            // `speed_lines` at the shipped width: one pixel at 720 rows.
            720,
            vec![Topology::Lines],
        );
        match e.fit {
            Err(Unfit::FlooringHidesTooMuch { share, allowed }) => {
                assert!(
                    (share - 0.75).abs() < 1e-9,
                    "an upper rung at half the target hides 3/4, not {share}"
                );
                assert_eq!(allowed, FLOORED_SHARE_ALLOWED);
            }
            other => panic!("expected a hidden-share refusal, got {other:?}"),
        }
        assert_eq!(e.ms(), None, "a refusal hands out no number");
    }

    /// **The shipped `Lines` pairing now has somewhere to stand.**
    /// `speed_lines` ships `width` at 0.00139 — its own comment calls it *one
    /// pixel at 720 rows* — so its floor is 720 rows and so is the reference
    /// target. Under ADR-0285 that was [`Unfit::NoRoomBelowTheTarget`]; under
    /// ADR-0293 the upper rung moves to [`upper_rung_rows`] and the pair is
    /// placed.
    ///
    /// **720 rather than 719**, which this said until ADR-0285 and which
    /// [`a_rate_becomes_the_height_at_which_it_is_one_pixel`] has always
    /// contradicted: `1 / 0.00139` is 719.4 and the floor rounds up.
    #[test]
    fn a_floor_at_the_target_moves_the_upper_rung_rather_than_refusing() {
        let floor = sub_pixel_floor_rows(0.001_39).expect("a positive rate has a floor");
        assert_eq!(floor, 720);
        let [low, high] = rungs(AT, floor).expect("the upper rung moves");
        assert_eq!(low, (320, 180));
        assert_eq!(high, (1109, 624));
        let share = floored_share(low, high, AT, floor);
        assert!(
            share <= FLOORED_SHARE_ALLOWED,
            "{share} is over the allowance the rung was placed to meet"
        );
    }

    /// **Every floor the shipped corpus states is placed now, and none of them
    /// hides more than the allowance.** This is
    /// `no_shipped_per_element_floor_leaves_room_under_the_reference_target`
    /// turned round: it was written the morning ADR-0285 landed to mechanise
    /// the finding that the whole per-element corpus refused, and ADR-0293 is
    /// what changed the finding.
    ///
    /// The floors are the eight in `crates/karakuri-ir/tests/rate.rs` taken
    /// over the declared ranges, plus `u32::MAX` for the four that cannot be
    /// bounded at all — which [`estimate`] passes as *not known* and which is
    /// the worst case rather than a sentinel.
    #[test]
    fn every_shipped_floor_is_placed_and_none_hides_more_than_the_allowance() {
        for floor in [715, 720, 1450, 4141, u32::MAX] {
            let [low, high] = rungs(AT, floor).unwrap_or_else(|e| panic!("{floor} rows: {e:?}"));
            assert_eq!((low, high), ((320, 180), (1109, 624)), "a floor of {floor}");
            let share = floored_share(low, high, AT, floor);
            assert!(
                (0.249_1..=FLOORED_SHARE_ALLOWED).contains(&share),
                "a floor of {floor} rows hides {share}"
            );
        }
        // And the cheap pair is still what a floor under the quarter-height
        // rung gets, with nothing hidden at all.
        let [low, high] = rungs(AT, 180).expect("180 is the quarter-height rung");
        assert_eq!((low, high), ((320, 180), PREPARATION_RESOLUTION));
        assert_eq!(floored_share(low, high, AT, 180), 0.0);
    }

    /// **The upper rung is where the allowance puts it, and no higher.**
    /// `ceil` rather than `round`, because rounding down would put the share
    /// over the allowance the rung exists to meet.
    #[test]
    fn the_accurate_upper_rung_is_the_lowest_one_inside_the_allowance() {
        for rows in [8u32, 63, 100, 360, 720, 1080, 2160] {
            let high = upper_rung_rows(rows);
            let share = 1.0 - (f64::from(high) / f64::from(rows)).powi(2);
            assert!(share <= FLOORED_SHARE_ALLOWED, "{rows} rows hides {share}");
            let under = 1.0 - (f64::from(high - 1) / f64::from(rows)).powi(2);
            assert!(
                under > FLOORED_SHARE_ALLOWED,
                "{rows} rows would still be inside the allowance one row lower"
            );
        }
        assert_eq!(upper_rung_rows(720), 624);
    }

    /// **A floor between the two rungs hides less than the peak**, because no
    /// primitive in the frame is small enough to reach it. That is the middle
    /// branch of [`floored_share`], and it is what makes the bound tight
    /// rather than merely sound.
    #[test]
    fn a_floor_the_frame_cannot_reach_hides_less_than_the_peak() {
        let (low, high) = ((320, 180), (1109, 624));
        let peak = floored_share(low, high, AT, u32::MAX);
        let near = floored_share(low, high, AT, 400);
        assert!(
            near < peak / 2.0,
            "a 400-row floor hides {near} against a worst case of {peak}"
        );
        // And a floor the cheap pair clears hides nothing whatever the rungs.
        assert_eq!(floored_share(low, high, AT, 180), 0.0);
    }

    /// **The correction covers what the flooring hides, on a frame built to
    /// hide the most it can.**
    ///
    /// The derivation in *How strictly the floor is read* is worth only what
    /// it is checked against, so this builds the adversary it claims to bound:
    /// a quarter of a million primitives all at the one size that maximises
    /// `u(x)/x²`, which is `x = q` — one pixel across at the upper rung, and
    /// `q` pixels at the target. Their coverage is computed with ADR-0245's
    /// rounding at each rung, turned into a millisecond figure at a made-up
    /// cost per covered pixel, fitted, and the answer compared with the truth.
    ///
    /// **Synthetic on purpose.** What is under test is the arithmetic of the
    /// bound; a real draw would be testing the machine.
    #[test]
    fn the_correction_covers_the_worst_frame_the_flooring_can_build() {
        const COUNT: f64 = 262_144.0;
        // Milliseconds per covered pixel, and the simulation-and-vertex term
        // that does not move with the target. Neither figure matters to the
        // bound; they are chosen so the frame is mostly fragment work, which
        // is what puts the undershoot near the whole of the share rather than
        // near a fraction of it — the bound is on the *fragment* cost.
        const PER_PIXEL: f64 = 2.0e-5;
        const INVARIANT: f64 = 0.5;

        let target = AT;
        let floor = u32::MAX;
        let [low, high] = rungs(target, floor).expect("the accurate pair");
        let rows = f64::from(target.1);
        // The worst primitive there is: one pixel across at the upper rung.
        let x = rows / f64::from(high.1);
        // ADR-0245: a primitive under a pixel is drawn at one and dimmed, so
        // its coverage stops falling.
        let coverage = |h: u32| {
            let across = x * f64::from(h) / rows;
            COUNT * across.max(1.0).powi(2)
        };
        let cost = |h: u32| INVARIANT + PER_PIXEL * coverage(h);

        let e = fit(
            measured(cost(low.1) as f32, low),
            measured(cost(high.1) as f32, high),
            target,
            floor,
            vec![Topology::Points],
        );
        let f = e.fit.expect("the accurate pair fits");
        let truth = cost(target.1);
        assert!(
            f64::from(f.ms) >= truth,
            "the corrected estimate is {:.3} ms against a truth of {truth:.3} ms, which is the \
             undershoot the correction exists to cover",
            f.ms
        );
        // And the raw fit is what it was rescued from: under-stating by very
        // nearly the whole share.
        let raw = f64::from(f.invariant_ms) + f64::from(f.fragment_ms);
        assert!(
            raw < truth,
            "the raw fit came out {raw:.3} ms and the truth is {truth:.3}, so this frame is not \
             the adversary it is written as"
        );
        let hid = 1.0 - raw / truth;
        let floored = e.floored.expect("a floored rung is on the record");
        assert!(
            hid <= floored.share + 1e-9,
            "the flooring hid {hid} where the bound says at most {}",
            floored.share
        );
        // **And the adversary is a real one**: it hides very nearly the whole
        // of what the bound allows, so the correction is being exercised at
        // its limit rather than against a frame that never needed it.
        assert!(
            hid > floored.share * 0.9,
            "this frame hid only {hid} of an allowed {}, so it is not the worst case",
            floored.share
        );
    }

    /// **A stroke floors in one dimension and is milder throughout**, which
    /// is why one rule answers for both and why the bound is stated for the
    /// sprite. A stroke's coverage is its length times its width and only the
    /// width floors, so under a pixel its coverage falls off linearly rather
    /// than stopping — and at the accurate pair its worst ratio is 0.056
    /// against a sprite's 0.249.
    ///
    /// The two functions here are `u(x)/max(x², 1)` for a sprite and its
    /// stroke counterpart, swept rather than solved: what is asserted is that
    /// [`floored_share`] bounds both.
    #[test]
    fn a_stroke_hides_less_than_a_sprite_and_both_are_inside_the_bound() {
        let target = AT;
        let [low, high] = rungs(target, u32::MAX).expect("the accurate pair");
        let rows = f64::from(target.1);
        let (p, q) = (rows / f64::from(low.1), rows / f64::from(high.1));
        let lambda = (area(target) - area(low)) / (area(high) - area(low));
        let bound = floored_share(low, high, target, u32::MAX);

        // A sprite `x` pixels across at the target: coverage `max((x/s)², 1)`
        // at the target divided by `s`, so the excess over the model is what
        // ADR-0245's rounding put there.
        let sprite = |x: f64| {
            let e = |s: f64| (1.0 - (x / s).powi(2)).max(0.0);
            e(1.0) + (lambda - 1.0) * e(p) - lambda * e(q)
        };
        // A stroke `y` pixels wide at the target, of unit length there: only
        // the width floors, so the coverage is `(1/s)·max(y/s, 1)`.
        let stroke = |y: f64| {
            let e = |s: f64| {
                let w = y / s;
                if w < 1.0 {
                    (1.0 - w) / s
                } else {
                    0.0
                }
            };
            e(1.0) + (lambda - 1.0) * e(p) - lambda * e(q)
        };

        let mut worst_sprite: f64 = 0.0;
        let mut worst_stroke: f64 = 0.0;
        for step in 1..=20_000u32 {
            let x = f64::from(step) / 2_000.0;
            worst_sprite = worst_sprite.max(sprite(x) / x.powi(2).max(1.0));
            worst_stroke = worst_stroke.max(stroke(x) / x.max(1.0));
        }
        assert!(
            worst_sprite <= bound + 1e-6,
            "a sprite hides {worst_sprite} where the bound says {bound}"
        );
        assert!(
            worst_stroke <= bound + 1e-6,
            "a stroke hides {worst_stroke} where the bound says {bound}"
        );
        assert!(
            worst_stroke < worst_sprite / 4.0,
            "a stroke floors in one dimension and should be far milder: \
             {worst_stroke} against {worst_sprite}"
        );
    }

    /// **A frame floored at the target as well costs the fit nothing.** `u(x)`
    /// is zero for `x <= 1`: the primitive is one pixel at both rungs *and* at
    /// the target, so it is a constant, and a fit that puts a constant in `a`
    /// predicts it exactly. This is the case ADR-0285's reading refused over —
    /// the primitive holding `soft_points` to a 4141-row floor is 0.17 pixels
    /// across at a 720-row target.
    #[test]
    fn a_primitive_floored_at_the_target_too_is_predicted_exactly() {
        const COUNT: f64 = 262_144.0;
        const PER_PIXEL: f64 = 2.0e-7;
        const INVARIANT: f64 = 3.0;

        let target = AT;
        let floor = 4141;
        let [low, high] = rungs(target, floor).expect("the accurate pair");
        // 0.17 pixels across at the target, which is one pixel everywhere.
        let cost = |_h: u32| INVARIANT + PER_PIXEL * COUNT;
        let e = fit(
            measured(cost(low.1) as f32, low),
            measured(cost(high.1) as f32, high),
            target,
            floor,
            vec![Topology::Points],
        );
        let f = e.fit.expect("a flat pair fits");
        let raw = f64::from(f.invariant_ms) + f64::from(f.fragment_ms);
        assert!(
            (raw - cost(target.1)).abs() < 1e-3,
            "the raw fit came out {raw:.4} ms against a truth of {:.4}",
            cost(target.1)
        );
    }

    /// The rungs are half and a quarter of the target's height, at the target's
    /// aspect ratio, and the upper one at the reference target is
    /// [`PREPARATION_RESOLUTION`] — which is what makes that constant derived
    /// rather than chosen.
    #[test]
    fn the_rungs_are_half_and_a_quarter_of_the_targets_height() {
        let [low, high] = rungs(AT, 1).expect("a fullscreen Set has no floor");
        assert_eq!(high, PREPARATION_RESOLUTION);
        assert_eq!(low, (320, 180));
        assert!(
            (area(high) / area(AT) - 0.25).abs() < 1e-9,
            "the upper rung is a quarter of the target's area"
        );
    }

    /// A floor between the two default rungs raises the lower one to it rather
    /// than refusing the pair. A rung *at* the floor is above it in the sense
    /// that matters: every primitive is still at least one pixel there.
    #[test]
    fn a_floor_between_the_rungs_raises_the_lower_one_to_it() {
        let [low, high] = rungs(AT, 250).expect("250 rows fits under 360");
        assert_eq!(low.1, 250);
        assert_eq!(high, PREPARATION_RESOLUTION);
        // And the aspect ratio is the target's, because the camera derives its
        // own from the viewport.
        assert_eq!(low, (444, 250));
    }

    /// **The floor a rate implies**, which is the language's arithmetic and not
    /// a measurement: a primitive is `rate * height` pixels across, so it
    /// reaches one pixel at `1 / rate` rows.
    #[test]
    fn a_rate_becomes_the_height_at_which_it_is_one_pixel() {
        assert_eq!(sub_pixel_floor_rows(0.001_39), Some(720));
        assert_eq!(sub_pixel_floor_rows(0.005_56), Some(180));
        assert_eq!(sub_pixel_floor_rows(0.000_69), Some(1450));
        assert_eq!(sub_pixel_floor_rows(0.0), None);
        assert_eq!(sub_pixel_floor_rows(f32::NAN), None);
    }

    fn bound(procedure: &str, bound: Bound) -> RateBound {
        RateBound {
            procedure: procedure.to_string(),
            bound,
        }
    }

    fn at_least(procedure: &str, rate: f32) -> RateBound {
        bound(
            procedure,
            Bound::AtLeast {
                rate,
                over: Vec::new(),
            },
        )
    }

    /// **A renderer with no primitive floors at one row**, which is what a
    /// fullscreen Set is made of. `karakuri_ir::check` infers
    /// [`Topology::Fullscreen`] from the missing `vertex` block and
    /// `karakuri_ir::rate` reads [`Bound::NoPrimitive`] off the same absence,
    /// so the two cannot disagree about it.
    #[test]
    fn a_set_that_draws_no_primitive_floors_at_one_row() {
        assert_eq!(floor_rows(&[bound("wash", Bound::NoPrimitive)]), Ok(1));
        assert_eq!(
            floor_rows(&[
                bound("wash", Bound::NoPrimitive),
                bound("haze", Bound::NoPrimitive)
            ]),
            Ok(1)
        );
        assert_eq!(floor_rows(&[]), Ok(1));
    }

    /// **The greatest of the renderers' floors, not the first.** The floor is
    /// the height at which the *smallest* primitive in the frame is still a
    /// pixel across, so the renderer drawing finest sets it for the whole Set —
    /// and a fullscreen one beside it does not pull it back down.
    #[test]
    fn a_sets_floor_is_the_finest_renderers() {
        // 0.004 is one pixel at 250 rows and 0.00139 at 720.
        let floor = floor_rows(&[
            at_least("dots", 0.004),
            bound("wash", Bound::NoPrimitive),
            at_least("streaks", 0.001_39),
        ]);
        assert_eq!(floor, Ok(720));
    }

    /// **One renderer nobody can bound refuses the whole Set.** A floor that
    /// holds for three renderers of four is not a floor: the fourth is drawing
    /// something no rung can be certified above.
    #[test]
    fn one_unbounded_renderer_refuses_the_set() {
        let floor = floor_rows(&[
            at_least("dots", 0.004),
            bound(
                "bloom",
                Bound::Unbounded {
                    at: karakuri_ir::Span::EMPTY,
                    lower: 0.0,
                },
            ),
        ]);
        assert_eq!(floor, Err(Unfit::FloorUnknown));
    }

    /// Two draws at one size are one measurement taken twice, and `b` would be
    /// `0/0`.
    #[test]
    fn two_rungs_at_one_size_are_refused() {
        let e = fit(
            measured(4.0, PREPARATION_RESOLUTION),
            measured(4.2, PREPARATION_RESOLUTION),
            AT,
            1,
            vec![Topology::Fullscreen],
        );
        assert!(matches!(e.fit, Err(Unfit::RungsCoincide { .. })));
    }

    /// **Two instruments are not two rungs.** `Probe::run` demotes itself to a
    /// host clock for life on the first implausible sample, so one probe can
    /// hand back a GPU number and then a host number — and the two are not
    /// comparable, which the probe's own module doc is emphatic about.
    #[test]
    fn a_pair_taken_by_two_instruments_is_refused() {
        let mut high = measured(6.0, PREPARATION_RESOLUTION);
        high.method = MeasurementMethod::GpuTimestamp;
        let e = fit(
            measured(4.0, (320, 180)),
            high,
            AT,
            1,
            vec![Topology::Fullscreen],
        );
        assert_eq!(e.fit, Err(Unfit::InstrumentsDiffer));
    }

    /// The instrument travels with the number, and says which direction it is
    /// wrong in. `P-0095` is what requires it of anything downstream of a
    /// measurement.
    #[test]
    fn an_estimate_says_which_clock_answered_and_that_it_reads_high() {
        let e = fit(
            measured(1.0, (320, 180)),
            measured(2.0, (640, 360)),
            AT,
            1,
            vec![Topology::Fullscreen],
        );
        assert_eq!(e.method(), Some(MeasurementMethod::HostWallClock));
        assert!(e.biased_high());
        assert_eq!(e.rungs.expect("recorded")[0].capacity, 262_144);
    }

    /// A target smaller than either rung is answered rather than clamped: with
    /// the two terms separated, only the fragment part shrinks, which is the
    /// prediction rather than a rounding. The console's preview cell is 112x63.
    #[test]
    fn a_target_smaller_than_the_rungs_shrinks_only_the_fragment_term() {
        let e = fit(
            measured(2.0, (320, 180)),
            measured(3.0, (640, 360)),
            (112, 63),
            1,
            vec![Topology::Fullscreen],
        );
        let f = e.fit.expect("a clean pair");
        assert!(
            f.ms > f.invariant_ms && f.ms < 2.0,
            "{:.3} ms should sit between the invariant term {:.3} and the smaller rung",
            f.ms,
            f.invariant_ms
        );
    }
}
