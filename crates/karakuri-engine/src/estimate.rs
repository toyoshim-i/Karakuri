//! **What a slot will cost at full size, from two small draws taken while it
//! primes.**
//!
//! A slot is drawn small while it prepares, and those small draws are a second
//! measurement — a higher-confidence estimate of what the same material costs
//! at full size, on this machine, rather than a number carried from elsewhere.
//! This module is the number. It does not draw the badge; the console does
//! that, and the five bands it reads the number into are
//! `docs/manual/console.html`.
//!
//! ## The rule, in one line
//!
//! **A frame is `a + b·area`, and two draws at two sizes are what separate the
//! two terms.** One draw gives their sum and no way to divide it.
//!
//! The form comes from the language rather than from a machine. A frame is
//! three parts: the simulation over `capacity`, the vertex stage per primitive,
//! and the fragment stage over covered pixels. The first two do not depend on
//! the target's size. The third does, and it does so for **every** topology,
//! because [`karakuri_ir::ast::Output::PointRate`] is a *fraction of the
//! render target's height* rather than a pixel count — a fullscreen pass covers
//! the target's area, a sprite covers `capacity × (rate × height)²`, a stroke
//! its length times its width, and every one of those is a fraction of a size
//! that moves.
//!
//! **[`Topology`] is therefore not the discriminator.** What decides whether a
//! procedure *looks* invariant or *looks* area-proportional is its coverage,
//! `capacity × rate²`, and a param moves that at any time: the same
//! `soft_points` sits on either side of any rule keyed on the enum, at
//! `point_scale` 0.00556 and at 0.0222. The split cannot be inferred from a
//! Set's shape, so it is measured. The enum is used here for exactly one thing
//! — whether there is a primitive at all, which decides whether ADR-0245's
//! sub-pixel floor exists — and [`floor_rows`] is the whole of that.
//!
//! ## What replaced what
//!
//! Until ADR-0266 this module scaled the *whole* measured cost by the ratio of
//! the areas, clamped at 1.0, one rule for every topology. That treats every
//! frame as if all of its cost were fragment work, so it multiplies `a` along
//! with `b·area`, and `a` is most of a per-element frame at the reference
//! capacity. Measured on one machine, the shipped corpus overshot by very
//! nearly the whole area ratio and every `Points` and `Lines` slot landed in
//! the badge's last band — the band that stops a slot. ADR-0266 carries the
//! figures, the alternatives, and why this is a fit rather than a per-topology
//! branch.
//!
//! ## What the language forces, and what is chosen
//!
//! **Forced.**
//!
//! - The cost has the form `a + b·area`, from the three parts above.
//! - `a ≥ 0` and `b ≥ 0`. Neither term is a quantity that can be negative, so a
//!   fit producing one is a failed measurement rather than a cheap Set, and is
//!   reported as [`Unfit::FragmentTermNegative`] or
//!   [`Unfit::InvariantTermNegative`] rather than clamped. Clamping would hand
//!   a caller a number produced by arithmetic it has no way to distrust.
//! - Both rungs sit above ADR-0245's sub-pixel floor. Below `1 / rate` rows a
//!   primitive is drawn at one pixel and dimmed rather than dropped, so its
//!   coverage stops being `(rate × height)²` and becomes 1 — **a different
//!   picture, not a smaller one**. A rung there reads *high* against the model,
//!   which drives `b` down and makes the estimate **undershoot**: the one
//!   direction *round the estimate toward refusing* forbids. Hence
//!   [`Unfit::RungBelowFloor`] and [`Unfit::NoRoomBelowTheTarget`].
//! - The two rungs are the target scaled by one factor in both dimensions. A
//!   Set's camera derives its aspect ratio from its viewport, so a rung at a
//!   different aspect ratio draws a different picture — the same failure as
//!   drawing below the floor, by a different route.
//!
//! **Chosen**, and marked so a wrong one is a one-clause revision rather than a
//! reopened decision (`docs/contributing.md` §4):
//!
//! - The rungs are the target at **half** and at **a quarter** of its height,
//!   so a quarter and a sixteenth of its area. Their **fragment** work is 5/16
//!   of a full-size draw's; their invariant work is paid twice, once per draw,
//!   because that is the term that does not shrink. The prediction reduces to
//!   `m_lo + 5·(m_hi − m_lo)`, so noise in either rung is amplified about
//!   fivefold into the answer — which is the price of putting the rungs low
//!   enough to be cheap, and the reason they are not put lower still. Rungs at
//!   a half and a quarter of the *area* would amplify threefold and put 3/4 of
//!   a full draw's fragment work through the probe.
//! - The lower rung is raised to the floor when the quarter-height rung would
//!   sit under it, rather than the pair being refused outright. A rung *at* the
//!   floor is above it in the sense that matters: every primitive is still at
//!   least one pixel there.
//!
//! ## What is not derivable here, and is refused rather than guessed
//!
//! **Nothing on the CPU side knows a Set's emitted `point_rate`.** It is an
//! expression evaluated per element in the vertex stage, over attributes this
//! side never reads back, so the floor `1 / rate` is not a number
//! [`estimate`] can compute. It does not guess one:
//!
//! - A Set whose renderers are all [`Topology::Fullscreen`] draws no primitive.
//!   `karakuri_ir::check` infers `Fullscreen` from the *absence* of a `vertex`
//!   block, and requires `point_rate` unconditionally when there is one, so
//!   "fullscreen" and "emits no rate" are the same fact. Its floor is one row.
//! - A Set that draws [`Topology::Points`] or [`Topology::Lines`] has a floor
//!   this module cannot see, and [`estimate`] answers [`Unfit::FloorUnknown`]
//!   without drawing anything. A caller that *does* know the smallest rate its
//!   material emits turns it into rows with [`sub_pixel_floor_rows`] and calls
//!   [`estimate_above_floor`].
//!
//! That is a refusal and not a workaround, and it is the honest reading of the
//! corpus: `speed_lines` ships `width` at 0.00139, one pixel at 720 rows, so
//! **its floor is the reference size itself** and there is no smaller draw that
//! says anything new about it. The area-ratio rule returned a number there
//! anyway. This one says it cannot.
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
//! ## The instrument travels with the number
//!
//! Both rungs come from **one** [`Probe`], resized between them.
//! [`Probe::run`] demotes itself to a host clock for life on the first
//! implausible sample, and this crate's own development machine makes it do so
//! every time (`docs/contributing.md` §1), so two probes can land on different
//! [`MeasurementMethod`]s and produce figures a fit would then be subtracting.
//! One probe can still demote *between* the two rungs, which is why
//! [`Unfit::InstrumentsDiffer`] exists: two numbers on two scales are not two
//! rungs of one curve.
//!
//! **A host-clock estimate is biased high, and both terms inherit it.** The
//! host clock brackets `queue.submit` and a `poll`, so it carries a
//! submit-and-wait round trip that the GPU never did: roughly constant in the
//! target's size, so it lands almost entirely in `a` and is added once to the
//! answer rather than multiplied by the area ratio. [`Estimate::biased_high`]
//! says so without a caller having to know any of that, and
//! [`Estimate::method`] is there for one that does — which is what `P-0095`
//! requires of anything downstream of a measurement.
//!
//! ## What estimating costs
//!
//! Two [`Probe::run`]s where there was one, at a quarter and a sixteenth of the
//! target's area. The fragment work goes from 4/16 of a full-size draw to 5/16
//! — the second rung is worth about a sixteenth of one full frame there — and
//! the **invariant** work is paid twice rather than once, which for
//! per-element material at the reference capacity is the larger half of the
//! bill. That is the price of separating `a` from `b` at all, and it is the
//! same fact the module is about: `a` does not get cheaper by drawing smaller.
//! The whole-capacity upload `Set::build` leaves staged and the second one
//! [`Set::rewind`] pays are unchanged — this module does not touch them.
//!
//! **The saving is in the fragments and nowhere else**, which is what the two
//! terms say it must be: material whose cost is not in its fragments costs the
//! same to measure small as at full size, because there were no fragments to
//! save. Measured on one machine at the reference capacity, `drift_shell +
//! soft_points` cost 8.87 ms at 1280x720 and 9.00 ms at 640x360 — a saving of
//! nothing, and a `b` of nothing to go with it — while `speed_lines` at the
//! declared maximum width cost 86.85 ms and 31.65 ms.

use karakuri_ir::Topology;

use crate::probe::{Measurement, MeasurementMethod, Probe};
use crate::set::Set;
use crate::swap::PROBE_STEPS;
use crate::Signals;

/// **The upper rung, for the reference target.**
///
/// [`rungs`] places the rungs at half and a quarter of the target's height, so
/// for `swap::PROBE_RESOLUTION` — 1280x720, the size every host-clock figure in
/// this repository is quoted at — the upper one is 640x360. It is **derived
/// rather than chosen**: this constant is what the rule yields there, kept
/// under a name because it is what the tests and the prose point at, and not a
/// size anything picks.
///
/// It was a chosen constant until ADR-0266, and what chose it was a knee in a
/// ladder measured on one machine — which is not a basis for a constant this
/// side of the workspace bakes in. ADR-0266 says why, and what replaced it.
pub const PREPARATION_RESOLUTION: (u32, u32) = (640, 360);

/// **The height at which a primitive of `rate` is one pixel across**, in rows,
/// rounded up.
///
/// `point_rate` is a fraction of the render target's height
/// ([`karakuri_ir::ast::Output::PointRate`]), so a primitive is `rate × height`
/// pixels across and reaches one pixel at `1 / rate` rows. Below that,
/// ADR-0245 draws it at one pixel and pays the difference in the colour rather
/// than dropping it — so the *picture* survives and the *coverage* stops
/// falling, which is exactly why a rung there cannot be fitted against one
/// above it.
///
/// A sprite floors in two dimensions and a stroke in one. Both stop getting
/// cheaper at the same height, so one function answers for both.
///
/// A `rate` that is not finite and positive has no floor to report and returns
/// [`None`].
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

/// **The floor under a Set that draws `topologies`**, in rows, or [`None`] when
/// this side cannot know it.
///
/// [`Topology::Fullscreen`] is inferred by `karakuri_ir::check` from a
/// procedure having no `vertex` block, and a `vertex` block must write
/// `point_rate` on every path — so a fullscreen renderer emits no rate, draws
/// no primitive, and has nothing that can fall under a pixel. Its floor is one
/// row.
///
/// Anything per-element emits a rate this side never sees, and the answer is
/// [`None`] rather than a guess. See *What is not derivable here* in the module
/// doc.
///
/// **This is the only thing [`Topology`] decides in this module**, and it is
/// not the `a`/`b` split: it is whether a floor exists at all.
pub fn floor_rows(topologies: &[Topology]) -> Option<u32> {
    if topologies.iter().all(|t| *t == Topology::Fullscreen) {
        Some(1)
    } else {
        None
    }
}

/// **Where the two draws are taken**, for a target and a floor. Low area first.
///
/// Half and a quarter of the target's height, both scaled by one factor so the
/// aspect ratio is the target's; the lower one raised to `floor` where the
/// quarter-height rung would sit under it. See *What the language forces, and
/// what is chosen* in the module doc.
///
/// **Pure arithmetic** — no device, no queue.
pub fn rungs(target: (u32, u32), floor: u32) -> Result<[(u32, u32); 2], Unfit> {
    let rows = target.1.max(1);
    let floor = floor.max(1);
    let high = (rows / 2).max(1);
    let low = (rows / 4).max(1).max(floor);
    if high < floor || low >= high {
        return Err(Unfit::NoRoomBelowTheTarget { floor, target });
    }
    Ok([at_rows(target, low), at_rows(target, high)])
}

/// `target` scaled to `rows` rows, keeping its aspect ratio and never reaching
/// zero in either dimension.
fn at_rows(target: (u32, u32), rows: u32) -> (u32, u32) {
    let height = u64::from(target.1.max(1));
    let width = (u64::from(target.0.max(1)) * u64::from(rows) / height).max(1);
    (width.min(u64::from(u32::MAX)) as u32, rows.max(1))
}

/// The area of a size, in pixels, as the fit's arithmetic sees it.
fn area((w, h): (u32, u32)) -> f64 {
    f64::from(w.max(1)) * f64::from(h.max(1))
}

/// A slot's cost at a target size, from a two-rung fit — **or why there is no
/// number**.
///
/// **Carries what produced it**, on the same terms as [`Measurement`] and for
/// the same reason (`P-0095`): a consumer that cannot see the instrument, the
/// sizes measured at and the two terms cannot check the number it was handed.
/// [`Estimate::ms`] alone is never enough to act on — read [`Estimate::method`]
/// and [`Estimate::biased_high`] first.
#[derive(Debug, Clone, PartialEq)]
pub struct Estimate {
    /// The size the estimate is *for* — the output's, not a probe's.
    ///
    /// A parameter rather than a constant because ADR-0246 makes the render
    /// size the output's, which makes an extrapolation a per-output question
    /// rather than a global one.
    pub target: (u32, u32),
    /// **What the Set draws**, in renderer order. A Set with no renderer cannot
    /// be built, so this is never empty. It decides [`Estimate::floor`] and
    /// nothing else — see the module doc on why it is not the discriminator.
    pub topologies: Vec<Topology>,
    /// The sub-pixel floor both rungs had to clear, in rows, or [`None`] when
    /// it was not knowable and nothing was drawn.
    pub floor: Option<u32>,
    /// The two draws, **low area first**, each with its own instrument,
    /// capacity and size on it. [`None`] when the rungs could not be placed, in
    /// which case nothing was drawn and nothing was spent.
    pub rungs: Option<[Measurement; 2]>,
    /// The fit, or why there is not one.
    pub fit: Result<Fit, Unfit>,
}

impl Estimate {
    /// **What one frame of this Set is estimated to cost at
    /// [`Estimate::target`]**, in milliseconds, or [`None`] when the fit
    /// failed. This is the number the console reads into the risk badge's five
    /// bands.
    pub fn ms(&self) -> Option<f32> {
        self.fit.as_ref().ok().map(|f| f.ms)
    }

    /// Which clock answered, or [`None`] when nothing was drawn.
    ///
    /// Both rungs are taken by one [`Probe`] and a fit with two methods on it
    /// is refused as [`Unfit::InstrumentsDiffer`], so where there is a fit
    /// there is one answer to this.
    pub fn method(&self) -> Option<MeasurementMethod> {
        self.rungs.as_ref().map(|[low, _]| low.method)
    }

    /// **True when the number is biased high** because the rungs came from a
    /// host clock rather than from GPU timestamps.
    ///
    /// A host measurement brackets `queue.submit` and a `poll`, so it includes
    /// a submit-and-wait round trip the GPU never spent. That cost does not
    /// move with the target, so it lands in [`Fit::invariant_ms`] and is added
    /// to the answer once. False when nothing was drawn: there is no number to
    /// be biased.
    pub fn biased_high(&self) -> bool {
        self.method() == Some(MeasurementMethod::HostWallClock)
    }
}

/// The two terms, solved for.
///
/// Both are non-negative by construction: a fit that produced a negative one is
/// an [`Unfit`] instead, because neither is a quantity that can be below zero
/// and a fit that says otherwise measured something other than what it thinks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    /// `a + b·area`, at [`Estimate::target`], in milliseconds.
    pub ms: f32,
    /// **`a`**: the part that does not move with the target — the simulation
    /// over `capacity`, the vertex stage per primitive, and on a host clock the
    /// submit-and-wait round trip. Milliseconds.
    pub invariant_ms: f32,
    /// **`b·area`** at [`Estimate::target`]: the fragment stage, in
    /// milliseconds. The part an output twice the size pays twice.
    pub fragment_ms: f32,
    /// **`b`**: milliseconds per pixel of target area. Kept as `f64` because it
    /// is a difference of two close numbers divided by a large one, and the
    /// caller that wants a millisecond figure wants [`Fit::fragment_ms`].
    pub ms_per_pixel: f64,
}

/// Why there is no number.
///
/// Every variant is a refusal to answer rather than a degraded answer, which is
/// `P-0095` at one remove: an estimate built on a measurement that cannot be
/// fitted is not a worse estimate, it is a different quantity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unfit {
    /// **The Set draws a per-element primitive and nothing here knows its
    /// rate.** ADR-0245 puts a floor at `1 / rate` rows and `point_rate` is a
    /// vertex-stage expression this side never evaluates, so no rung can be
    /// certified above it. Nothing was drawn. A caller that knows the smallest
    /// rate its material emits has [`estimate_above_floor`].
    FloorUnknown,
    /// **Two rungs above the floor and below the target do not both fit.** The
    /// shipped `speed_lines` pairing is this case at the reference size: its
    /// floor is 719 rows and the target is 720, so there is nowhere below the
    /// target that is not also below the floor. Nothing was drawn.
    NoRoomBelowTheTarget {
        /// The floor that left no room, in rows.
        floor: u32,
        /// The target the rungs were being placed under.
        target: (u32, u32),
    },
    /// A rung was handed to [`fit`] below the floor it was measured against.
    /// [`rungs`] never produces one; a caller supplying its own can.
    RungBelowFloor {
        /// The offending rung.
        rung: (u32, u32),
        /// The floor it sits under, in rows.
        floor: u32,
    },
    /// The two rungs have the same area, so `b` is `0/0`. Two draws at one size
    /// are one measurement taken twice.
    RungsCoincide {
        /// The area both rungs share, in pixels.
        area: f64,
    },
    /// **`b < 0`: the draw got dearer as the target got smaller.** A fragment
    /// stage cannot cost less than nothing, so this is a failed measurement and
    /// not a cheap Set. It is what a tile-based renderer's binning does to a
    /// quarter of a million primitives sorted into too few tiles, and it is
    /// what thermal drift between the two rungs looks like from here.
    FragmentTermNegative {
        /// The `b` that came out, in milliseconds per pixel.
        ms_per_pixel: f64,
    },
    /// **`a < 0`: the fit puts the size-invariant part below zero.** The
    /// simulation and the vertex stage cannot cost less than nothing, so the
    /// two rungs did not lie on one line and something moved between them.
    InvariantTermNegative {
        /// The `a` that came out, in milliseconds.
        invariant_ms: f32,
    },
    /// The two rungs were taken by different instruments, so they are two
    /// numbers on two scales rather than two points on one curve. A single
    /// [`Probe`] can do this to itself: [`Probe::run`] demotes to a host clock
    /// for life on the first implausible sample.
    InstrumentsDiffer,
    /// The arithmetic did not produce a finite number.
    NotFinite,
}

/// **Solve `a + b·area` from two measurements and evaluate it at `target`.**
/// Pure arithmetic — no device, no queue, and the whole of the rule.
///
/// The rungs may be given in either order; they are sorted by area, and the
/// [`Estimate`] records them low area first. `floor` is the sub-pixel floor
/// they were placed against, in rows, and both are checked against it —
/// [`rungs`] places them so they pass, and this is what holds a caller that
/// places its own.
///
/// **The target is not checked against the floor**, and that is deliberate: a
/// target *below* the floor draws floored primitives whose coverage has stopped
/// falling, so `a + b·area` over-states it, and over-stating is the direction
/// *round the estimate toward refusing* asks for. The console's preview cell is
/// 112x63 and "what would this slot cost drawn into one" is a question somebody
/// will eventually ask this function.
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
    let refuse = |why: Unfit| Estimate {
        target,
        topologies: topologies.clone(),
        floor: Some(floor),
        rungs: Some([low, high]),
        fit: Err(why),
    };

    if low.method != high.method {
        return refuse(Unfit::InstrumentsDiffer);
    }
    for rung in [low.resolution, high.resolution] {
        if rung.1 < floor {
            return refuse(Unfit::RungBelowFloor { rung, floor });
        }
    }
    let (a_low, a_high) = (area(low.resolution), area(high.resolution));
    if a_low == a_high {
        return refuse(Unfit::RungsCoincide { area: a_low });
    }

    // `b` is the slope through the two rungs and `a` is where that line meets
    // zero area. Both in `f64`: the numerator is a difference of two close
    // millisecond figures and the denominator is a pixel count.
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
    let ms = a + fragment;
    if !ms.is_finite() {
        return refuse(Unfit::NotFinite);
    }
    Estimate {
        target,
        topologies,
        floor: Some(floor),
        rungs: Some([low, high]),
        fit: Ok(Fit {
            ms: ms as f32,
            invariant_ms: a as f32,
            fragment_ms: fragment as f32,
            ms_per_pixel: b,
        }),
    }
}

/// **Draw `set` at two small sizes and say what it would cost at `target`.**
///
/// The measuring half of what `Priming` now means (ADR-0053 gave it the other
/// half, warming buffers). The floor comes from [`floor_rows`], so a Set that
/// draws anything per-element is answered [`Unfit::FloorUnknown`] **without
/// being drawn at all** — see *What is not derivable here* in the module doc,
/// and [`estimate_above_floor`] for the caller that knows the rate.
///
/// Everything else is [`estimate_above_floor`]'s, including the restoration of
/// the Set and the handling of the probe.
pub fn estimate(
    probe: &mut Probe,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    set: &mut Set,
    target: (u32, u32),
) -> Estimate {
    let topologies = set.drawn_topologies();
    match floor_rows(&topologies) {
        Some(floor) => estimate_above_floor(probe, device, queue, set, target, floor),
        None => Estimate {
            target,
            topologies,
            floor: None,
            rungs: None,
            fit: Err(Unfit::FloorUnknown),
        },
    }
}

/// **[`estimate`], for a caller that knows the floor.**
///
/// `floor` is the height in rows at which the smallest primitive this Set draws
/// is still one pixel across — [`sub_pixel_floor_rows`] turns a rate into one.
/// Pass 1 for material with no primitive. Getting it wrong in the low direction
/// is the failure ADR-0245 describes: a rung under the true floor measures a
/// different picture, drives `b` down, and makes the answer undershoot.
///
/// What is timed is one frame at [`PROBE_STEPS`] simulation steps, at the Set's
/// real capacity and with its real parameters, at each of [`rungs`]'s two
/// sizes — the same content as [`crate::swap::measure`], at a quarter and a
/// sixteenth of the target's area.
///
/// **Never on the render thread**, exactly as [`crate::swap::measure`] is not:
/// it submits and waits once per sample, now twice over. And **destructive on a
/// Set that has stepped**, for the same reason and by the same mechanism — it
/// steps the Set and then [`Set::rewind`]s it, which restores what `Set::build`
/// left rather than what this call found. A Set is primed before it is on air,
/// which is where this belongs.
///
/// `probe` is taken rather than constructed, and this **resizes it** rather
/// than making a second one: [`Probe::run`] demotes itself to a host clock for
/// life on the first implausible sample, and two probes can land on different
/// [`MeasurementMethod`]s and produce figures a fit would then be subtracting.
/// [`Probe::resize`] replaces the attachment and leaves the verdict alone.
/// **The probe is left at the upper rung** — the larger of the two, so a caller
/// alternating with [`crate::swap::measure`] pays the smaller reallocation.
///
/// **The lower rung is measured first.** Drift between the two rungs lands
/// entirely in `b`: a device that warms across the pair reads the second rung
/// high, which raises `b` and over-states the answer, and a cold first sample
/// does the same. Both round toward refusing. The reverse order would round the
/// other way.
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
        // The uniforms have never been written otherwise, and the aspect ratio
        // the camera derives comes from the viewport set just above — so each
        // measured frame is preceded by a real `prepare`, exactly as
        // `swap::measure`'s is and for the reason stated there.
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
    use crate::swap::PROBE_RESOLUTION;

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
        let b = 9.2 / area(PROBE_RESOLUTION);
        let (low, high) = ((320, 180), (640, 360));
        let e = fit(
            measured((a + b * area(low)) as f32, low),
            measured((a + b * area(high)) as f32, high),
            PROBE_RESOLUTION,
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
        assert_eq!(e.target, PROBE_RESOLUTION);
    }

    /// **The rungs may arrive in either order**, and the [`Estimate`] records
    /// them low area first whichever way they came.
    #[test]
    fn the_rungs_are_sorted_by_area_rather_than_trusted() {
        let (low, high) = ((320, 180), (640, 360));
        let e = fit(
            measured(4.0, high),
            measured(2.0, low),
            PROBE_RESOLUTION,
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
            PROBE_RESOLUTION,
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
            PROBE_RESOLUTION,
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
            PROBE_RESOLUTION,
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

    /// **A rung under ADR-0245's floor is refused rather than fitted.** It
    /// measures a different picture — every primitive rounded up to one pixel
    /// and dimmed — so the pair is not two points on one curve.
    #[test]
    fn a_rung_below_the_sub_pixel_floor_is_refused() {
        let e = fit(
            measured(4.0, (320, 180)),
            measured(6.0, (640, 360)),
            PROBE_RESOLUTION,
            // `speed_lines` at the shipped width: one pixel at 719 rows.
            719,
            vec![Topology::Lines],
        );
        match e.fit {
            Err(Unfit::RungBelowFloor { rung, floor }) => {
                assert_eq!(rung, (320, 180));
                assert_eq!(floor, 719);
            }
            other => panic!("expected a below-floor refusal, got {other:?}"),
        }
    }

    /// **The shipped `Lines` pairing has nowhere to stand.** `speed_lines`
    /// ships `width` at 0.00139 — its own comment calls it *one pixel at 720
    /// rows* — so its floor is 719 rows and the reference target is 720. There
    /// is no rung below the target that is not also below the floor, and
    /// nothing is drawn.
    #[test]
    fn a_floor_at_the_target_leaves_no_room_for_two_rungs() {
        let why = rungs(PROBE_RESOLUTION, 719).expect_err("719 rows under a 720-row target");
        assert_eq!(
            why,
            Unfit::NoRoomBelowTheTarget {
                floor: 719,
                target: PROBE_RESOLUTION
            }
        );
    }

    /// The rungs are half and a quarter of the target's height, at the target's
    /// aspect ratio, and the upper one at the reference target is
    /// [`PREPARATION_RESOLUTION`] — which is what makes that constant derived
    /// rather than chosen.
    #[test]
    fn the_rungs_are_half_and_a_quarter_of_the_targets_height() {
        let [low, high] = rungs(PROBE_RESOLUTION, 1).expect("a fullscreen Set has no floor");
        assert_eq!(high, PREPARATION_RESOLUTION);
        assert_eq!(low, (320, 180));
        assert!(
            (area(high) / area(PROBE_RESOLUTION) - 0.25).abs() < 1e-9,
            "the upper rung is a quarter of the target's area"
        );
    }

    /// A floor between the two default rungs raises the lower one to it rather
    /// than refusing the pair. A rung *at* the floor is above it in the sense
    /// that matters: every primitive is still at least one pixel there.
    #[test]
    fn a_floor_between_the_rungs_raises_the_lower_one_to_it() {
        let [low, high] = rungs(PROBE_RESOLUTION, 250).expect("250 rows fits under 360");
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

    /// **[`Topology`] decides whether a floor exists, and nothing else.** A
    /// fullscreen renderer emits no `point_rate` — `karakuri_ir::check` infers
    /// the topology from the absence of the `vertex` block that would have to
    /// write one — so there is no primitive to fall under a pixel. Anything
    /// per-element has a floor this side cannot see.
    #[test]
    fn only_a_set_with_no_primitive_has_a_floor_this_side_can_state() {
        assert_eq!(floor_rows(&[Topology::Fullscreen]), Some(1));
        assert_eq!(
            floor_rows(&[Topology::Fullscreen, Topology::Fullscreen]),
            Some(1)
        );
        assert_eq!(floor_rows(&[Topology::Points]), None);
        assert_eq!(floor_rows(&[Topology::Lines]), None);
        assert_eq!(floor_rows(&[Topology::Fullscreen, Topology::Points]), None);
    }

    /// Two draws at one size are one measurement taken twice, and `b` would be
    /// `0/0`.
    #[test]
    fn two_rungs_at_one_size_are_refused() {
        let e = fit(
            measured(4.0, PREPARATION_RESOLUTION),
            measured(4.2, PREPARATION_RESOLUTION),
            PROBE_RESOLUTION,
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
            PROBE_RESOLUTION,
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
            PROBE_RESOLUTION,
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
