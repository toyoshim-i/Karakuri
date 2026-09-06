//! **What a slot will cost at full size, from a draw taken while it primes.**
//!
//! `docs/roadmap.md`, *The preparation slot is the measurement*: a slot is
//! drawn small while it prepares, and that small draw is a second measurement —
//! a higher-confidence estimate of what the same material costs at full size,
//! on this machine, rather than a number carried from elsewhere. This module is
//! the number. It does not draw the badge; the console does that, and the five
//! bands it reads the number into are `docs/manual/console.html`.
//!
//! ## The rule, in one line
//!
//! **The measurement is scaled by the ratio of the target's area to the
//! probe's, whatever the procedure draws.**
//!
//! Not because every cost is fill-bound — it is not — but because a single
//! draw cannot separate the two terms it is made of. A frame is
//! `a + b * area`: `a` is the simulation and the vertex stage, invariant in the
//! target's size, and `b * area` is the fragment stage. One measurement gives
//! their sum at one area and no way to divide it. Of the two extrapolations
//! that could be applied to the sum, the area-proportional one is always the
//! larger — the target only ever grows from the probe's size — so it is the one
//! that satisfies the roadmap's **round the estimate toward refusing**.
//!
//! [`karakuri_ir::Topology`] therefore does **not** select a rule here. It is
//! recorded on the [`Estimate`] because it is what an overshoot has to be read
//! against, and for no other reason.
//!
//! ## The check: fragment work does track area, and a frame is not fragments
//!
//! `examples/small_draw.rs` measured each material at eight target sizes with
//! one `Probe`, interleaved, first pass discarded, median of two, on a
//! `HostWallClock` — which `docs/contributing.md` says is what this machine's
//! GPU timestamps demote to. **A later run of the same file was discarded on
//! its spread**: the `speed_lines` rung at the declared maximum width is 87 ms
//! a frame, sixteen samples of it heats the machine, and that run's passes
//! disagreed by up to 3x (87.092 to 266.480 ms on one rung). Every figure below
//! is from a run whose spreads were inside 2%. Fitting `a + b * area` over the
//! rungs that are **above the floor**:
//!
//! | material | topology | `a` | `b * A₇₂₀` | 1280x720 | fit |
//! |---|---|---|---|---|---|
//! | `field_march` | `Fullscreen` | 0.52 ms | 3.05 ms | 3.569 ms | R² = 0.999 over all 8 |
//! | `soft_points` @ `point_scale` 0.0222 | `Points` | 9.0 ms | 9.2 ms | 18.052 ms | R² = 0.97 over the top 5 |
//! | `speed_lines` @ `width` 0.0333 | `Lines` | 19.3 ms | 66.7 ms | 86.852 ms | least squares over 3 |
//!
//! **`b` is real and positive for all three, so the area proportionality is
//! confirmed** — as the language says it must be, `point_rate` being a fraction
//! of the target's height rather than a pixel count. That is the `Lines` check
//! the maintainer asked for and it passes: at the declared maximum stroke width
//! the fragment term is 67 ms of an 87 ms frame, and quartering the area took
//! the frame from 86.85 ms to 31.65 ms.
//!
//! **`a` is what the rule cannot see.** It is the simulation and the vertex
//! stage — 262144 elements' worth — plus the host clock's own submit-and-wait,
//! which `flat_fill` (an L4 that writes a constant and runs no simulation)
//! measured at 0.41 to 0.48 ms at every size. Scaling the *sum* by the area
//! ratio scales `a` with it, and `a` is most of a per-element frame at the
//! reference capacity. **The overshoot is the price, and it is measured**:
//! `measured_small * area_ratio` against the same material at 1280x720, same
//! probe, same run.
//!
//! | material | topology | from 905x509 (2x) | from 640x360 (4x) | from 320x180 (16x) |
//! |---|---|---|---|---|
//! | `field_march` | `Fullscreen` | 1.13x | 1.48x | 3.11x |
//! | `speed_lines` @ max width | `Lines` | — | 1.46x | 4.96x |
//! | `soft_points` @ 0.0222 | `Points` | 1.54x | 2.58x | 9.00x |
//! | `soft_points`, shipped | `Points` | 2.01x | 4.06x | 15.3x |
//! | `speed_lines`, shipped | `Lines` | 2.05x | 3.90x | 16.6x |
//!
//! The first three rows are material measured **above its floor** and the
//! overshoot there is 1.1x to 2.6x at a 2x or 4x ratio — safe, and usable for
//! `Fullscreen`. The last two rows are the shipped corpus, and their overshoot
//! is very nearly the area ratio itself, which is what a flat curve produces.
//! Against the console's bands — green under 4 ms, red about 12, purple over
//! 16, and *purple is a slot that gets stopped* — `soft_points` at 8.87 ms
//! estimates as 36 ms from 640x360. **Every `Points` and `Lines` slot in
//! `examples/` is stopped by this rule at any small size.**
//!
//! Whether that is the floor or the `a` term is the one thing worth being
//! precise about, because they suggest different fixes. It is **both, and the
//! `a` term is the larger**: `soft_points` at `point_scale` 0.0222 is above its
//! floor at every rung down to 320x180 and still overshoots 2.6x from 640x360,
//! because 9.0 ms of its 18.1 ms frame does not move when the target does.
//!
//! **This module ships the decided rule and does not work around it.** No
//! per-topology branch and no second rung. `P-0095` is why the price is stated
//! here rather than smoothed over.
//!
//! ## Where the small draw may be taken, and why not smaller
//!
//! Two separate floors, and [`PREPARATION_RESOLUTION`] sits under one of them
//! knowingly.
//!
//! **The floor ADR-0245 puts under the cost.** A primitive is `rate * height`
//! pixels across — `point_rate` is a fraction of the target's height, not a
//! pixel count — so it reaches one pixel at `1 / rate` rows, and below that it
//! is drawn at one pixel rather than dropped. A sprite floors in two dimensions
//! and a stroke in one, so a stroke keeps its length and only stops thinning;
//! both stop getting cheaper. **The larger of the two floors is the stroke's,
//! and for this corpus it is 719 rows** — `speed_lines` ships `width` at
//! 0.00139, which its own comment calls *one pixel at 720 rows*. So there is no
//! small size at which the shipped line pairing is above its floor, and the
//! ladder shows exactly that: `warp_tunnel + speed_lines` measured between 6.21
//! and 6.72 ms at every size from 1280x720 down to 112x63.
//!
//! | material and setting | emitted rate | floor, in rows |
//! |---|---|---|
//! | `soft_points`, fastest elements | 0.00556 | 180 |
//! | `soft_points`, slowest elements | 0.00195 | 514 |
//! | `drift_streaks`, fastest | 0.00167 | 599 |
//! | `speed_lines` | 0.00139 | 719 |
//! | `drift_streaks`, slowest | 0.00058 | 1711 |
//! | any of them, at the declared minimum | 0.00069 | 1449 |
//!
//! **A second floor the ladder found and nothing had written down.** Below
//! 640x360 the measured cost of dense element material stops falling and starts
//! *rising*: `soft_points` at `point_scale` 0.0222 and the reference capacity
//! measured 9.08 ms at 452x254, 10.17 at 320x180, 12.10 at 226x127 and 13.64 at
//! 160x90 — with sprites 10, 4, 2.8 and 2 pixels across, so none of it is
//! ADR-0245's floor. A quarter of a million primitives binned into 14400 texels
//! is a tile-based renderer's worst case, and this crate's development machine
//! is one. **A draw made smaller past 640x360 is not merely uninformative, it
//! is more expensive.**
//!
//! [`PREPARATION_RESOLUTION`] is therefore 640x360: half the reference in each
//! dimension, an area ratio of exactly 4, and the smallest rung on the ladder
//! at which no material measured *more* than it did one rung up. **It is
//! knowingly below the stroke floor of the shipped `speed_lines` pairing**,
//! because 1280x720 is too, and there is nowhere below the reference that is
//! not. Where the corpus is above its floor, the rule behaves; where it is not,
//! the small draw says nothing new and the rule returns the area ratio.
//!
//! ## What estimating costs now, against what it cost before
//!
//! One [`Probe::run`] and nothing else. The whole-capacity upload `Set::build`
//! leaves staged and the second one [`Set::rewind`] pays are unchanged — this
//! module does not touch them — so the difference is the draw, and the draw at
//! a quarter of the area:
//!
//! | material | 1280x720 | 640x360 | what the small draw saves |
//! |---|---|---|---|
//! | `drift_shell` + `soft_points` | 8.87 ms | 9.00 ms | nothing |
//! | `warp_tunnel` + `speed_lines` | 6.47 ms | 6.31 ms | nothing |
//! | `field_march` | 3.57 ms | 1.32 ms | 63% |
//! | `soft_points` at `point_scale` 0.0222 | 18.05 ms | 11.66 ms | 35% |
//! | `speed_lines` at `width` 0.0333 | 86.85 ms | 31.65 ms | 64% |
//!
//! **The saving is exactly where the roadmap wanted it and nowhere else.**
//! "Trialling something extremely heavy does little harm" holds — the 86.85 ms
//! pairing is measured for 31.65 — and material whose cost is not in its
//! fragments costs the same to measure small as to measure at full size,
//! because there were no fragments to save.

use karakuri_ir::Topology;

use crate::probe::{Measurement, Probe};
use crate::set::Set;
use crate::swap::PROBE_STEPS;
use crate::Signals;

/// **The size a slot is drawn at while it primes.**
///
/// Half of [`PROBE_RESOLUTION`] in each dimension, so the area ratio back to
/// the reference is exactly 4 and an extrapolation to it is one multiply with
/// no rounding of its own. See "Where the small draw may be taken" in the
/// module doc for the two floors this is chosen against — it is above the
/// ladder's measured knee and below `speed_lines`'s sub-pixel floor, and the
/// second of those is a property of the corpus rather than of this constant.
pub const PREPARATION_RESOLUTION: (u32, u32) = (640, 360);

/// A slot's cost at a target size, extrapolated from a small draw.
///
/// **Carries what produced it**, on the same terms as [`Measurement`] and for
/// the same reason (`P-0095`): a consumer that cannot see the instrument, the
/// size measured at and the factor applied cannot check the number it was
/// handed. [`Estimate::ms`] alone is never enough to act on — read
/// [`Measurement::method`] on [`Estimate::measured`] first.
#[derive(Debug, Clone, PartialEq)]
pub struct Estimate {
    /// **What one frame of this Set is estimated to cost at
    /// [`Estimate::target`]**, in milliseconds. This is the number the console
    /// reads into the risk badge's five bands.
    ///
    /// **Rounded toward refusing, and the rounding is the rule itself**: the
    /// area factor is applied to a measurement that is partly invariant in the
    /// target's size, so this is an upper bound rather than a prediction, and
    /// the module doc measures by how much.
    pub ms: f32,
    /// The small draw this was extrapolated from, with its own instrument,
    /// capacity and resolution on it.
    pub measured: Measurement,
    /// The size the estimate is *for* — the output's, not the probe's.
    pub target: (u32, u32),
    /// `target` area over `measured.resolution` area: the factor applied, and
    /// the factor by which a cost that does not scale with area is overstated.
    /// Never below 1.0 — see [`extrapolate`].
    pub area_ratio: f32,
    /// **What the Set draws**, in renderer order. Recorded rather than acted
    /// on: one rule covers every topology, and this is here so an overshoot can
    /// be read against the thing that explains it. A Set with no renderer
    /// cannot be built, so this is never empty.
    pub topologies: Vec<Topology>,
}

/// Scale a small draw to a target size. **Pure arithmetic** — no device, no
/// queue, and the whole of the rule.
///
/// `target` is the output's size — a parameter rather than a constant because
/// `docs/adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md`
/// makes the render size the output's, which makes an extrapolation a
/// per-output question rather than a global one.
///
/// **The factor is clamped at 1.0.** A target smaller than the probe's would
/// otherwise scale the measurement *down*, which is the one direction this must
/// never round: a slot that turns out to be cheaper than measured costs
/// nothing, and one that turns out to be dearer is a dropped frame in front of
/// an audience. It is not a hypothetical shape — the console's preview cell is
/// 112x63 — and "estimate the cost of drawing this into a cell" is a question
/// somebody will eventually ask this function.
pub fn extrapolate(
    measured: Measurement,
    target: (u32, u32),
    topologies: Vec<Topology>,
) -> Estimate {
    let area = |(w, h): (u32, u32)| f64::from(w.max(1)) * f64::from(h.max(1));
    let ratio = (area(target) / area(measured.resolution)).max(1.0);
    Estimate {
        ms: (f64::from(measured.ms) * ratio) as f32,
        measured,
        target,
        area_ratio: ratio as f32,
        topologies,
    }
}

/// **Draw `set` small, once, and say what it would cost at `target`.**
///
/// The measuring half of what `Priming` now means (ADR-0053 gave it the other
/// half, warming buffers). What is timed is one frame at [`PROBE_STEPS`]
/// simulation steps, at the Set's real capacity and with its real parameters,
/// into a [`PREPARATION_RESOLUTION`] target — the same content as
/// [`crate::swap::measure`], at a quarter of the area.
///
/// **Never on the render thread**, exactly as [`crate::swap::measure`] is not:
/// it submits and waits once per sample. And **destructive on a Set that has
/// stepped**, for the same reason and by the same mechanism — it steps the Set
/// and then [`Set::rewind`]s it, which restores what `Set::build` left rather
/// than what this call found. A Set is primed before it is on air, which is
/// where this belongs.
///
/// `probe` is taken rather than constructed, and this **resizes it** rather
/// than making a second one: [`Probe::run`] demotes itself to a host clock for
/// life on the first implausible sample, and two probes can land on different
/// [`MeasurementMethod`](crate::probe::MeasurementMethod)s and produce figures
/// a budget would then be summing. [`Probe::resize`] replaces the attachment
/// and leaves the verdict alone. **The probe is left at the small size**, so a
/// caller alternating with [`crate::swap::measure`] pays one reallocation each
/// way; a caller doing a deck's worth of slots pays it once.
pub fn estimate(
    probe: &mut Probe,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    set: &mut Set,
    target: (u32, u32),
) -> Estimate {
    let capacity = set.capacity();
    let viewport = set.viewport();
    let topologies = set.drawn_topologies();
    probe.resize(device, PREPARATION_RESOLUTION);
    set.resize(device, PREPARATION_RESOLUTION.0, PREPARATION_RESOLUTION.1);
    // The uniforms have never been written otherwise, and the aspect ratio the
    // camera derives comes from the viewport set just above — so the measured
    // frame is preceded by a real `prepare`, exactly as `swap::measure`'s is
    // and for the reason stated there.
    set.prepare(queue, PROBE_STEPS, &Signals::default());
    let measured = probe.run(device, queue, set, PROBE_STEPS, capacity);
    set.rewind(device, queue);
    set.resize(device, viewport.0, viewport.1);
    extrapolate(measured, target, topologies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::MeasurementMethod;
    use crate::swap::PROBE_RESOLUTION;

    fn measured(ms: f32, resolution: (u32, u32)) -> Measurement {
        Measurement {
            ms,
            method: MeasurementMethod::HostWallClock,
            capacity: 262_144,
            resolution,
        }
    }

    /// The rule, on the arithmetic the ladder was read with: a quarter-area
    /// draw is multiplied by four.
    #[test]
    fn a_small_draw_is_scaled_by_the_ratio_of_the_areas() {
        let e = extrapolate(
            measured(9.0, PREPARATION_RESOLUTION),
            PROBE_RESOLUTION,
            vec![Topology::Points],
        );
        assert_eq!(e.area_ratio, 4.0);
        assert_eq!(e.ms, 36.0);
    }

    /// **The overshoot, as a test rather than as a paragraph.** `soft_points`
    /// at the reference capacity measured 8.872 ms at 1280x720 and 8.996 ms at
    /// 640x360 in the same process on one probe, so the rule turns a slot that
    /// costs 8.9 ms into an estimate of 36 ms — past the last of the console's
    /// five bands, which is the band that stops a slot.
    ///
    /// Here so that the finding cannot be lost by someone reading only the
    /// code, and so that a future rule which fixes it fails this test loudly
    /// rather than quietly changing what the number means.
    #[test]
    fn the_rule_overstates_material_whose_cost_is_not_in_its_fragments() {
        let e = extrapolate(
            measured(8.996, PREPARATION_RESOLUTION),
            PROBE_RESOLUTION,
            vec![Topology::Points],
        );
        let truth = 8.872;
        assert!(
            e.ms / truth > 4.0,
            "the measured overshoot was 4.06x and this says {:.2}x",
            e.ms / truth
        );
        assert!(
            e.ms > 16.0,
            "8.9 ms of `Points` has to land in the band that stops a slot for \
             this finding to be what it is, and it estimated {:.2} ms",
            e.ms
        );
    }

    /// A target smaller than the probe's does not scale the estimate down. The
    /// console's preview cell is 112x63 and asking what a slot costs drawn into
    /// one is a reachable question; answering it with a thirty-first of the
    /// measurement would be the one rounding this must never do.
    #[test]
    fn a_target_smaller_than_the_draw_does_not_round_the_estimate_down() {
        let e = extrapolate(
            measured(9.0, PREPARATION_RESOLUTION),
            (112, 63),
            vec![Topology::Points],
        );
        assert_eq!(e.area_ratio, 1.0);
        assert_eq!(e.ms, 9.0);
    }

    /// The instrument travels with the number. A consumer handed an
    /// [`Estimate`] can still ask which clock answered and at what size, which
    /// is what `P-0095` requires of anything downstream of a measurement.
    #[test]
    fn an_estimate_carries_the_measurement_that_produced_it() {
        let m = measured(1.32, PREPARATION_RESOLUTION);
        let e = extrapolate(m, PROBE_RESOLUTION, vec![Topology::Fullscreen]);
        assert_eq!(e.measured, m);
        assert_eq!(e.measured.method, MeasurementMethod::HostWallClock);
        assert_eq!(e.measured.resolution, PREPARATION_RESOLUTION);
        assert_eq!(e.target, PROBE_RESOLUTION);
    }

    /// **The one case the rule is good at**, kept beside the one it is bad at
    /// so the two are read together. `field_march` measured 3.569 ms at
    /// 1280x720 and 1.322 ms at 640x360, so the estimate is 5.29 ms against a
    /// truth of 3.57 — a 1.48x overshoot, and all of it the invariant term,
    /// which for a fullscreen procedure is the host clock's own round trip.
    #[test]
    fn a_fullscreen_procedure_is_the_case_the_rule_fits() {
        let e = extrapolate(
            measured(1.322, PREPARATION_RESOLUTION),
            PROBE_RESOLUTION,
            vec![Topology::Fullscreen],
        );
        let overshoot = e.ms / 3.569;
        assert!(
            (1.4..1.6).contains(&overshoot),
            "the measured overshoot was 1.48x and this says {overshoot:.2}x"
        );
    }
}
