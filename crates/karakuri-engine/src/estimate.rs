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
//! Set's shape, so it is measured. **The enum is now reported and read by
//! nothing here**: it decided one thing — whether there is a primitive at all
//! — and `karakuri_ir::rate` answers that from the same fact the check pass
//! infers the enum from, a `vertex` block that is not there. It is on
//! [`Estimate::topologies`] so a number can be read against what was drawn.
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
//! - ADR-0245's sub-pixel floor is **bounded and paid**, not merely avoided.
//!   Below `1 / rate` rows a primitive is drawn at one pixel and dimmed rather
//!   than dropped, so its coverage stops being `(rate × height)²` and becomes
//!   1 — **a different picture, not a smaller one**. A rung there reads *high*
//!   against the model, which drives `b` down and makes the estimate
//!   **undershoot**: the one direction *round the estimate toward refusing*
//!   forbids. What is forced is that the undershoot cannot exceed a share this
//!   module can state and divide back out — see *How strictly the floor is
//!   read*, [`FLOORED_SHARE_ALLOWED`] and ADR-0293. Where it would,
//!   [`Unfit::FlooringHidesTooMuch`] and [`Unfit::NoRoomBelowTheTarget`].
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
//! - **When the floor clears the half-height rung too, the pair moves rather
//!   than the estimate refusing.** The upper rung goes to
//!   [`upper_rung_rows`] — the lowest height at which the flooring can hide no
//!   more than [`FLOORED_SHARE_ALLOWED`] of the target's fragment cost — and
//!   the lower one stays at a quarter, which is cheap and keeps the two rungs
//!   far enough apart to subtract. At the reference target that is 624 rows
//!   against 180, so the probe pays 0.81 of a full draw's fragment work rather
//!   than 0.31, and the answer amplifies noise 1.7-fold rather than 9-fold.
//!   **What that costs is not what it looks like for the material that needs
//!   it.** Measured, `drift_shell + soft_points` cost 8.87 ms at 1280x720 and
//!   9.00 ms at 640x360 — a fragment term of nothing — so 0.81 of a full
//!   draw's fragments is 0.81 of nothing, and what is paid twice either way is
//!   the invariant term. It is not a rule: the floor is the *greatest* of the
//!   renderers' floors, so one hairline renderer beside a fullscreen one puts
//!   the whole Set down this branch.
//!
//! ## Where the floor comes from
//!
//! **Nothing on the CPU side evaluates a Set's `point_rate`**, and nothing
//! needs to. It is an expression over params, attributes and ambients, and
//! `karakuri_ir::rate::point_rate_bound` bounds it from below by interval
//! arithmetic over its params' **declared** ranges — the file's own statement
//! of how far a fader may take them — leaving everything else unbounded. The
//! smallest rate a renderer can emit becomes the greatest floor it can put
//! under a rung, and [`floor_rows`] takes the greatest over the Set's
//! renderers.
//!
//! **Over the declared range, and that is not the reading the rule asks for.**
//! ADR-0282 settled which state a number about a Set is taken over: a declared
//! value is the value in the untouched state, and where somebody moved one the
//! held value *is* the value. `karakuri_ir::rate::point_rate_bound_at` takes a
//! bound that way and `Set` holds every number it needs, but `Set::rate_bounds`
//! is computed once at build over the whole declared range and there is no
//! second reading beside it. What that costs, measured, is in the table below;
//! ADR-0293 says what closing it needs and why the loosened reading makes the
//! difference matter much less than it did.
//!
//! Going stale when a fader moves is **what the narrower reading is for**, not
//! a hazard: a write invalidates the estimate standing on it and the estimate
//! is taken again. `Set::rate_bound_contradicted` is the detection, and
//! [`Floor::Analysed`]'s `contradicted` carries it — under this module's
//! reading it makes the floor *unknown* rather than making the estimate
//! refused, because an unknown floor is a placement and no longer a refusal.
//!
//! **Where it cannot bound, the floor is unknown and not zero.** A rate that
//! reaches zero has no floor at all, and one nothing bounds below has none
//! either; [`floor_rows`] answers [`Unfit::FloorUnknown`] for both, with the
//! working on [`Estimate::floor_from`], and [`estimate`] reads that as *the
//! greatest floor there is* rather than as a refusal — which is sound, because
//! the bound below holds whatever rates the material emits. The record is
//! `docs/adr/0285-a-renderers-floor-is-bounded-from-its-declared-ranges-or-refused.md`,
//! and ADR-0293 for what is now done with the number.
//!
//! ## How strictly the floor is read
//!
//! **The condition is not that no primitive is floored. It is that what the
//! flooring can hide is a share this module can state and divide back out.**
//! ADR-0285 enforced the first and refused the whole shipped per-element
//! corpus for it; ADR-0293 is why the second is the right reading and this is
//! the arithmetic behind it.
//!
//! Take one primitive that is `x` pixels across at the target, rungs at
//! `H/p` and `H/q` rows, and `λ = (A − A_lo) / (A_hi − A_lo)`, which is the
//! factor [`fit`] extrapolates by. Its coverage is `max((x·h/H)², 1)` at every
//! height, so the coverage the fit predicts for it at the target, less the
//! coverage it really has there, is
//!
//! ```text
//! u(x) = 0        for x ≤ 1        floored at both rungs and at the target too
//! u(x) = x² − 1   for 1 ≤ x ≤ q    floored at both rungs, not at the target
//! u(x) = (λ−1)·(1 − x²/p²)  for q ≤ x ≤ p     floored at the lower rung only
//! u(x) = 0        for x ≥ p        floored nowhere
//! ```
//!
//! Two things fall out of it, and they are the whole of the change.
//!
//! **A primitive floored at the target as well costs the fit nothing.** It is
//! one pixel at both rungs *and* at the target, so it is a constant, and a fit
//! that puts a constant in `a` predicts it exactly. That is the case ADR-0285's
//! reading was refusing over: the primitive holding `soft_points` down to a
//! 4141-row floor is 0.17 pixels across at a 720-row target and is in this
//! class.
//!
//! **What it can cost is bounded without knowing a thing about the rates.**
//! `u(x)/max(x², 1)` peaks at `x = q`, where it is `1 − 1/q²`. So the share of
//! the target's fragment cost the flooring can hide is at most `1 − 1/q²` —
//! and `q` is the ratio of the target's height to the **upper rung's**, and
//! nothing else enters: not the lower rung, not the distribution of rates, not
//! the capacity. Where the floor is known it is tighter still, because no
//! primitive is smaller than `H / floor` pixels at the target and the peak may
//! be out of reach; [`floored_share`] is both cases.
//!
//! **The same number twice.** `1 − 1/q²` is also the share of a full-size
//! draw's fragment work the upper rung declines to measure, `q` being a
//! height ratio and the coverage going as its square. **The error a fit can
//! hide is the work it did not do** — so this is not a tolerance to be picked
//! but a price, and [`upper_rung_rows`] is where it is paid.
//!
//! **And the answer is divided by what is left.** The truth is at most
//! `1 / (1 − share)` times the fit, so [`Fit::ms`] carries that factor and the
//! estimate still rounds toward refusing. [`Estimate::floored`] is `Some` when
//! it was applied, because an estimate taken under a floored rung is not the
//! same statement as one taken clear of it.
//!
//! ## What this can and cannot answer for
//!
//! **Fifteen L4 procedures ship in `examples/`, and after ADR-0293 the floor
//! refuses none of them at the reference target.** Under ADR-0285's reading it
//! refused twelve: three draw no primitive and were answered as they always
//! were, eight stated a floor and every one of those floors was above the
//! half-height rung, and four could not be bounded at all. The bounds are
//! written out in `crates/karakuri-ir/tests/rate.rs`; what the floor does with
//! them is here, and `estimate_over_the_shipped_corpus` in
//! `tests/estimate.rs` is the same table mechanised.
//!
//! Two readings sit side by side, because `Set::rate_bounds` takes the wider
//! one and ADR-0282 asks for the narrower: **declared** is over the whole
//! declared range of every param, **as it stands** is over the value each param
//! is holding, which for an untouched Set is its declared default.
//!
//! | renderer | floor, declared | placed | floor, as it stands | placed |
//! |---|---|---|---|---|
//! | `field_lens`, `field_march`, `glow_march` | no primitive | clear | no primitive | clear |
//! | `plain_points` | 715 | 0.2492 | 240 | **clear** |
//! | `star_flares` | 715 | 0.2492 | 132 | **clear** |
//! | `sheet_shade` | 720 | 0.2492 | 720 | 0.2492 |
//! | `speed_lines` | 1450 | 0.2492 | 720 | 0.2492 |
//! | `soft_points`, `second_eye` | 4141 | 0.2492 | 514 | **0.1618** |
//! | `glass_shell` | 4141 | 0.2492 | 343 | **clear** |
//! | `drift_streaks` | 4141 | 0.2492 | 1711 | 0.2492 |
//! | `strand_strokes` | not known | 0.2492 | 164 | **clear** |
//! | `hard_dots`, `beat_strokes`, `beat_bloom` | not known | 0.2492 | not known | 0.2492 |
//!
//! *Clear* means both rungs sit above the floor, so no primitive is rounded up
//! at either and the fit is exact in ADR-0245's terms — [`Estimate::floored`]
//! is [`None`]. A share means the lower rung is under the floor, the upper rung
//! has moved to [`upper_rung_rows`], and [`Fit::ms`] carries the matching
//! correction: 1.3319 at 0.2492, and 1.1930 at 0.1618.
//!
//! **The tally.** Over the declared range, 3 of the 15 are clear and 12 are
//! floored; over the state as it stands, 7 are clear and 8 are floored, two of
//! those at the smaller share. **Nothing is refused either way**, where
//! ADR-0285 refused twelve. So the narrower reading is worth four procedures'
//! worth of exactness — and one procedure's worth of a bound at all
//! (`strand_strokes`, whose `width_var` is declared up to 1.0 and held at 0.45)
//! — and it is **no longer worth an answer**, which is the whole difference
//! this module's loosening makes to ADR-0285's *three of the eight would place
//! rungs at 720 rows instead of none*.
//!
//! The four the analysis cannot bound are worth naming, because none of them
//! is a failure of the analysis — and none of them is a refusal any more
//! either, an unknown floor being the greatest floor there is rather than a
//! missing one:
//!
//! - `hard_dots` — `dot_scale * size`, and `size` is an attribute whatever the
//!   simulation put in it. There is no declaration to read.
//! - `beat_strokes` — `... * (1.0 + age)`, and `age` is an attribute. The same.
//! - `beat_bloom` — `width * max(spill, age * glitch_glow)`, where `spill` is a
//!   `pow` that genuinely reaches zero and `glitch_glow` may be zero too. The
//!   rate really does reach zero, and a rate of zero has no floor.
//! - `strand_strokes` — `point_scale * (1.0 - width_var + width_var *
//!   hash1(..) * 2.0)`, and `width_var` is declared up to 1.0, where the first
//!   term is zero and `hash1` may be zero with it. The same over the
//!   declaration; over the held 0.45 it bounds at 0.0061.
//!
//! A Set whose primitives can be zero across draws nothing for those elements,
//! and no height makes them a pixel — which under the reading in *How strictly
//! the floor is read* costs the fit nothing at all: `u(0) = 0`.
//!
//! **The rates the material actually emits**, measured, are what the floor is
//! conservative against. These are per element rather than per procedure — the
//! same Set emits several — and the floor is [`sub_pixel_floor_rows`] of the
//! rate beside it:
//!
//! | material and setting | emitted rate | floor, in rows | pixels at a 720-row target |
//! |---|---|---|---|
//! | `soft_points`, fastest elements | 0.00556 | 180 | 4.0 |
//! | `soft_points`, slowest elements | 0.00195 | 513 | 1.4 |
//! | `drift_streaks`, fastest | 0.00167 | 599 | 1.2 |
//! | `speed_lines` | 0.00139 | 720 | 1.0 |
//! | `drift_streaks`, slowest | 0.00058 | 1725 | 0.42 |
//!
//! The last column is `x` in *How strictly the floor is read*, and it is what
//! says the shipped material sits in the band the correction is sized for: one
//! row is under a pixel at the target and costs the fit nothing, and the rest
//! are between one and four.
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

use karakuri_ir::rate::{Bound, RateBound};
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

/// **The share of a target's fragment cost a fit is allowed to hide.**
///
/// ADR-0245's flooring lets a two-rung fit under-state, and *How strictly the
/// floor is read* in the module doc bounds by how much: at most `1 − 1/q²` of
/// the target's fragment cost, `q` being the target's height over the upper
/// rung's. [`fit`] divides the answer by `1 − share`, so the estimate still
/// rounds toward refusing; what remains to be fixed is how large a share is
/// worth paying for.
///
/// **A quarter, and it is derived from the bands the number feeds.** The
/// console reads an estimate into five bands whose boundaries are 4, 8, 12 and
/// 16 ms — four slots to a 16.7 ms frame — and a value on a boundary rounds to
/// the worse band (`docs/manual/console.html`). So a correction of
/// `1/(1 − 1/4) = 4/3` moves a slot by at most one band anywhere on that scale,
/// and the binding case is the last boundary: 12 ms corrected is 16 ms, exactly
/// the boundary of the band that stops a slot. A fifth more would put a slot
/// the truth leaves in red into purple on the correction alone, and the number
/// would be doing the badge's deciding for it.
///
/// **It is not a tolerance, it is a price**, and [`upper_rung_rows`] is where
/// it is paid: `1 − 1/q²` is equally the share of a full draw's fragment work
/// the upper rung declines to measure. Buying a smaller correction means
/// drawing nearer to full size.
pub const FLOORED_SHARE_ALLOWED: f64 = 0.25;

/// **Where the upper rung goes when the floor is under it**, in rows.
///
/// The lowest height at which the flooring can hide no more than
/// [`FLOORED_SHARE_ALLOWED`] — `ceil(rows · √(1 − allowed))`, rounded **up**
/// because rounding down would put the share over the allowance. At the
/// reference target's 720 rows that is 624, which hides 0.2492.
///
/// Pure arithmetic, and the only thing it is a function of is the target: the
/// bound in *How strictly the floor is read* does not depend on the floor, on
/// the lower rung, or on anything about the material.
pub fn upper_rung_rows(rows: u32) -> u32 {
    let rows = rows.max(1);
    let wanted = f64::from(rows) * (1.0 - FLOORED_SHARE_ALLOWED).sqrt();
    (wanted.ceil() as u32).clamp(1, rows)
}

/// **How much of the target's fragment cost ADR-0245's flooring can hide from
/// a fit through these two rungs**, as a share between 0 and 1.
///
/// The derivation is *How strictly the floor is read* in the module doc, and
/// this is the three cases it ends in. `x` is the smallest primitive in the
/// frame measured in pixels across **at the target** — `target rows / floor` —
/// so a large floor is a small `x`, and `p` and `q` are the target's height
/// over the lower and the upper rung's.
///
/// - `x ≥ p`: nothing is floored at either rung and the fit is exact. **Zero.**
/// - `x ≤ q`: the worst primitive is at or below the peak of `u(x)/x²`, so the
///   bound is the peak itself, `1 − 1/q²`.
/// - between: the peak is out of reach, and the bound falls away as
///   `(λ−1)·(1 − x²/p²)/x²` — where `λ` is [`fit`]'s own extrapolation factor,
///   so the two cannot disagree about the line.
///
/// **The rungs' heights and not their areas**, because the flooring is about a
/// primitive's size across; `λ` is taken from the areas because that is what
/// the fit is solved in. For a pair at the target's aspect ratio — which is
/// what [`rungs`] places, and what the module doc's *forced* list requires —
/// the two agree.
pub fn floored_share(low: (u32, u32), high: (u32, u32), target: (u32, u32), floor: u32) -> f64 {
    let rows = f64::from(target.1.max(1));
    let p = rows / f64::from(low.1.max(1));
    let q = rows / f64::from(high.1.max(1));
    // An upper rung at the target extrapolates nothing, so there is nothing
    // for the flooring to hide behind.
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
        // No extrapolation, or none this function can read. The peak of the
        // rising branch is the only claim left that does not rest on `λ`.
        return (1.0 - 1.0 / (q * q)).clamp(0.0, 1.0);
    }
    let falling = |x: f64| (lambda - 1.0) * (1.0 - x * x / (p * p)) / (x * x);
    if x > q {
        return falling(x).clamp(0.0, 1.0);
    }
    // **The greater of the two branches at their meeting point**, rather than
    // the rising branch's `1 - 1/q²` alone. The two are equal for rungs whose
    // areas are exactly the target's times the square of their height ratio,
    // and `at_rows` truncates a width to an integer — so a rung can be a hair
    // smaller in area than its height implies, which lifts `λ` and with it the
    // falling branch. Taking the greater is what keeps this an upper bound
    // rather than a nearly-right one.
    (1.0 - 1.0 / (q * q)).max(falling(q)).clamp(0.0, 1.0)
}

/// **The floor under a Set whose renderers bound their rates like this**, in
/// rows, or [`Unfit::FloorUnknown`] when one of them could not.
///
/// **The greatest of the per-renderer floors**, because the floor is the height
/// at which the *smallest* primitive in the frame is still a pixel across and
/// one renderer drawing finer than another sets it for both. Each renderer's is
/// [`sub_pixel_floor_rows`] of the least rate
/// [`karakuri_ir::rate::point_rate_bound`] could prove for it.
///
/// [`Bound::NoPrimitive`] contributes one row and not a refusal: a procedure
/// with no `vertex` block emits no `point_rate`, draws no primitive, and has
/// nothing that can fall under a pixel. That is the same fact
/// [`Topology::Fullscreen`] reports — `karakuri_ir::check` infers the topology
/// from the absence this bound is read from — so a Set of nothing but
/// fullscreen renderers answers one row here as it always did.
///
/// **One [`Bound::Unbounded`] refuses the whole Set.** A floor that holds for
/// three renderers of four is not a floor: the fourth is drawing something
/// this side cannot certify a rung above.
pub fn floor_rows(bounds: &[RateBound]) -> Result<u32, Unfit> {
    let mut floor = 1;
    for bound in bounds {
        match bound.bound {
            Bound::NoPrimitive => {}
            Bound::AtLeast { rate, .. } => match sub_pixel_floor_rows(rate) {
                Some(rows) => floor = floor.max(rows),
                // `Bound::AtLeast` promises a finite positive rate, so this is
                // unreachable — and it is a refusal rather than an assertion
                // because what the caller is owed is a floor or a reason, and
                // an analysis that produced neither has produced a reason.
                None => return Err(Unfit::FloorUnknown),
            },
            Bound::Unbounded { .. } => return Err(Unfit::FloorUnknown),
        }
    }
    Ok(floor)
}

/// **Where [`Estimate::floor`] came from**, which is what `P-0095` asks of it:
/// a floor is the thing every rung was placed against, so a consumer that
/// cannot see how it was arrived at cannot check the rungs either.
#[derive(Debug, Clone, PartialEq)]
pub enum Floor {
    /// **The caller stated it** — [`estimate_above_floor`] — and nothing here
    /// checked it. Getting it wrong in the low direction is the failure
    /// ADR-0245 describes, and this variant is the record that nothing stood
    /// between the caller and it.
    Stated,
    /// **Read off the Set's own `point_rate` expressions**, one bound per L4
    /// procedure in `L4:n` order, of which the floor is the greatest.
    ///
    /// The bounds are over the params' **declared** ranges rather than the
    /// values the Set is holding, so the floor holds while an operator moves a
    /// fader. Nothing clamps a write to a declared range, and `contradicted`
    /// is the held value that falsified one — the name, the value, and the
    /// declaration it is outside of. `Some` here means the floor was not used.
    Analysed {
        /// One per L4 procedure, in the order `L4:n` addresses them.
        bounds: Vec<RateBound>,
        /// A held value outside the declaration a bound was taken over.
        contradicted: Option<(String, f32, [f32; 2])>,
    },
}

/// **That the lower rung sat under the floor, and what was done about it.**
///
/// [`Floor`] says where the floor came from; this says how strictly it was
/// read, which is the other half of what `P-0095` asks of the number. An
/// estimate taken with every primitive at least a pixel across at both rungs
/// and one taken with some of them rounded up are **not the same statement**,
/// and [`Estimate::floored`] is `None` for the first.
///
/// The arithmetic is *How strictly the floor is read* in the module doc and
/// [`floored_share`]; ADR-0293 is the record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Floored {
    /// **The share of the target's fragment cost the flooring can hide**. A
    /// bound and not a measurement: what it actually hid may be nothing at all.
    ///
    /// At most [`FLOORED_SHARE_ALLOWED`] wherever there is a fit, because a
    /// pair that would hide more is [`Unfit::FlooringHidesTooMuch`] — which
    /// carries this same number, and on whose [`Estimate`] this field is what
    /// the rungs *would* have hidden.
    pub share: f64,
    /// **What [`Fit::ms`] was multiplied by**, which is `1 / (1 - share)`. The
    /// truth is at most this times the raw fit, so applying it is what keeps
    /// the estimate rounding toward refusing.
    pub correction: f64,
}

/// **Where the two draws are taken**, for a target and a floor. Low area first.
///
/// **The cheap pair, where the floor allows it.** Half and a quarter of the
/// target's height, both scaled by one factor so the aspect ratio is the
/// target's, the lower one raised to `floor` where the quarter-height rung
/// would sit under it. Nothing is floored at either rung, so the fit is exact
/// in ADR-0245's terms and [`floored_share`] of the pair is zero.
///
/// **The accurate pair, where it does not.** A floor above the half-height
/// rung used to be [`Unfit::NoRoomBelowTheTarget`] and is the whole of the
/// shipped per-element corpus (ADR-0285). Here the upper rung moves to
/// [`upper_rung_rows`] instead — far enough up that the flooring can hide no
/// more than [`FLOORED_SHARE_ALLOWED`], which [`fit`] then divides back out —
/// and the lower one stays at a quarter of the target's height, where it is
/// cheap and far enough from the upper rung to subtract against.
///
/// **`floor` is only asked which of the two.** The bound the accurate pair
/// rests on holds for any rate whatever, so `u32::MAX` — [`estimate`]'s
/// spelling of *the floor is not known* — is a placement and not a refusal.
///
/// What is left to refuse is a target too short to hold two distinct rungs
/// under it at all. That is only reachable down the second branch, where the
/// upper rung rounds up onto the target itself: at [`FLOORED_SHARE_ALLOWED`],
/// a target under eight rows whose floor clears the half-height rung.
///
/// **Pure arithmetic** — no device, no queue. See *What the language forces,
/// and what is chosen* in the module doc, and ADR-0293.
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
    // **`upper_rung_rows` answers in heights and the allowance is a fact about
    // the pair as placed**, which is integers: `at_rows` truncates a width, so
    // a rung can be a hair under the area its height implies and hide a hair
    // more than the allowance. One row up is enough at every size in the
    // corpus and the search is what makes the guarantee hold rather than very
    // nearly hold.
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
    /// **Where that floor came from**, whether or not there was one. See
    /// [`Floor`]: the whole of what makes the number above checkable.
    pub floor_from: Floor,
    /// **How strictly that floor was read**, which is the other half of the
    /// same question. [`None`] means both rungs cleared the floor and no
    /// primitive was rounded up at either — ADR-0285's reading, and the one
    /// under which the fit is exact. [`Some`] means the lower rung sat under
    /// it, and carries the share the flooring can hide and the factor
    /// [`Fit::ms`] was multiplied by to cover it.
    pub floored: Option<Floored>,
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
    /// `a + b·area` at [`Estimate::target`], in milliseconds, **times
    /// [`Floored::correction`] where there was one**.
    ///
    /// So this is [`Fit::invariant_ms`] plus [`Fit::fragment_ms`] where
    /// [`Estimate::floored`] is [`None`], and strictly more than their sum
    /// where it is not: the other three fields are the line that was fitted
    /// and this is the answer that line supports, which under a floored rung
    /// is the higher of the two.
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
    /// **A renderer's `point_rate` could not be bounded away from zero**, so
    /// there is no height at which every primitive it draws is a pixel across
    /// and no rung can be certified above ADR-0245's floor. Nothing was drawn.
    ///
    /// **Which renderer, and what was proved of it anyway, is on
    /// [`Estimate::floor_from`]** rather than here — a
    /// [`karakuri_ir::rate::Bound::Unbounded`] entry among its bounds, or a
    /// `contradicted` value outside the declaration a bound was taken over.
    /// This variant stays a bare word because it is the *answer*, and the
    /// working is a different question with a longer reply.
    ///
    /// A caller that knows the smallest rate its material emits — from a
    /// measurement, or from a narrower reading than the declared ranges — has
    /// [`estimate_above_floor`].
    ///
    /// **[`floor_rows`]'s answer and no longer [`estimate`]'s.** ADR-0293 made
    /// an unknown floor a rung placement rather than a refusal: the share the
    /// flooring can hide is bounded whatever the rates are, so *not known* is
    /// read as the greatest floor there is and paid for like any other.
    FloorUnknown,
    /// **Two distinct rungs do not fit under the target at all.**
    ///
    /// It used to be what every per-element Set in `examples/` answered at the
    /// reference size — ADR-0285's floors run from 715 rows to 4141 against an
    /// upper rung of 360 — and ADR-0293 moved the upper rung instead. What
    /// reaches this now is a target too short to hold a pair down the accurate
    /// branch, where the upper rung rounds up onto the target itself: under
    /// eight rows at [`FLOORED_SHARE_ALLOWED`]. Nothing was drawn.
    NoRoomBelowTheTarget {
        /// The floor that left no room, in rows.
        floor: u32,
        /// The target the rungs were being placed under.
        target: (u32, u32),
    },
    /// **The rungs would let ADR-0245's flooring hide more of the target's
    /// fragment cost than [`FLOORED_SHARE_ALLOWED`].**
    ///
    /// The correction that would make such a fit sound is bigger than a whole
    /// band of the badge it feeds, so what came back would be a refusal
    /// wearing a number's clothes. [`rungs`] never produces such a pair — it
    /// moves the upper rung until the share is inside the allowance — so this
    /// is for a caller placing its own, which is what [`estimate_above_floor`]
    /// and [`fit`] are for.
    ///
    /// It replaces the flat *a rung below the floor is refused*: a rung under
    /// the floor is now a question of how much, and the module doc's *How
    /// strictly the floor is read* is the arithmetic.
    FlooringHidesTooMuch {
        /// What [`floored_share`] answered for the pair.
        share: f64,
        /// [`FLOORED_SHARE_ALLOWED`], so a reader has both halves.
        allowed: f64,
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
    // **How much of the answer the sub-pixel floor can hide**, from where the
    // rungs sit rather than from what the material is — see *How strictly the
    // floor is read*. Zero where both rungs clear the floor, which is the
    // exact case and the one [`rungs`] reaches first. Computed before anything
    // else because it is a fact about the placement, and a refusal is owed it
    // as much as a fit is.
    let share = floored_share(low.resolution, high.resolution, target, floor);
    let floored = (share > 0.0).then(|| Floored {
        share,
        correction: 1.0 / (1.0 - share),
    });
    let refuse = |why: Unfit| Estimate {
        target,
        topologies: topologies.clone(),
        floor: Some(floor),
        // The floor arrived as a number and this function has no way back to
        // what produced it. [`estimate`] replaces this on the way out.
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
    // **The correction is the whole of what a floored rung costs**, and it is
    // applied to the answer rather than to either term: the undershoot is in
    // the fragment work, but the fit misattributes part of it to `a`, so the
    // sound statement is about the sum. `a >= 0` is what makes it sound.
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

/// **Draw `set` at two small sizes and say what it would cost at `target`** —
/// with no floor from the caller, because the Set states one.
///
/// The measuring half of what `Priming` now means (ADR-0053 gave it the other
/// half, warming buffers).
///
/// **The floor is [`floor_rows`] over `Set::rate_bounds`**, which is
/// `karakuri_ir::rate`'s static bound on each renderer's `point_rate` against
/// its params' declared ranges. A renderer whose rate that analysis cannot hold
/// above zero, and a Set holding a value outside a declaration one of the
/// bounds was taken over, both leave the floor **unknown** — see [`Floor`] for
/// the working, which travels on [`Estimate::floor_from`] either way.
///
/// **An unknown floor is not a refusal.** It is passed to [`rungs`] as the
/// greatest floor there is, which places the accurate pair, and the answer
/// carries the correction that placement earns. That is ADR-0293 replacing
/// ADR-0285's `Unfit::FloorUnknown`, and it is sound because the share the
/// flooring can hide does not depend on the rates at all — see *How strictly
/// the floor is read* in the module doc.
///
/// **What the floor still decides is the placement and the statement.** Where
/// it clears the quarter-height rung the cheap pair is drawn, nothing is
/// rounded up at either rung, the fit is exact and [`Estimate::floored`] is
/// [`None`]. Where it does not, the upper rung moves, the probe costs about
/// two and a half times as much fragment work, and the answer is corrected.
/// [`Estimate::floor`] is [`None`] when the floor was not knowable at all.
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
    // **A held value outside a declaration falsifies the bound taken over it**,
    // and a bound that is not true of the run is not a floor. It is not a
    // refusal either: an unknown floor is *the greatest floor there is*, which
    // is a placement.
    let known = if contradicted.is_some() {
        None
    } else {
        floor_rows(bounds).ok()
    };
    // **`u32::MAX` is how *not known* is spelled to [`rungs`]**, and it is the
    // sound reading rather than a sentinel: a floor of `u32::MAX` rows says
    // every primitive is under a pixel at any size anybody will draw, which is
    // the worst case the bound in *How strictly the floor is read* covers.
    let mut e = estimate_above_floor(probe, device, queue, set, target, known.unwrap_or(u32::MAX));
    // **The floor came from the Set and the record has to say so.**
    // `estimate_above_floor` is the caller-stated path and marks every answer
    // it builds [`Floor::Stated`]; this is the one call site that knows better,
    // and the one that can say the floor was not knowable at all.
    e.floor = known;
    e.floor_from = from;
    e
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
            PROBE_RESOLUTION,
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
        let [low, high] = rungs(PROBE_RESOLUTION, floor).expect("the upper rung moves");
        assert_eq!(low, (320, 180));
        assert_eq!(high, (1109, 624));
        let share = floored_share(low, high, PROBE_RESOLUTION, floor);
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
            let [low, high] =
                rungs(PROBE_RESOLUTION, floor).unwrap_or_else(|e| panic!("{floor} rows: {e:?}"));
            assert_eq!((low, high), ((320, 180), (1109, 624)), "a floor of {floor}");
            let share = floored_share(low, high, PROBE_RESOLUTION, floor);
            assert!(
                (0.249_1..=FLOORED_SHARE_ALLOWED).contains(&share),
                "a floor of {floor} rows hides {share}"
            );
        }
        // And the cheap pair is still what a floor under the quarter-height
        // rung gets, with nothing hidden at all.
        let [low, high] = rungs(PROBE_RESOLUTION, 180).expect("180 is the quarter-height rung");
        assert_eq!((low, high), ((320, 180), PREPARATION_RESOLUTION));
        assert_eq!(floored_share(low, high, PROBE_RESOLUTION, 180), 0.0);
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
        let peak = floored_share(low, high, PROBE_RESOLUTION, u32::MAX);
        let near = floored_share(low, high, PROBE_RESOLUTION, 400);
        assert!(
            near < peak / 2.0,
            "a 400-row floor hides {near} against a worst case of {peak}"
        );
        // And a floor the cheap pair clears hides nothing whatever the rungs.
        assert_eq!(floored_share(low, high, PROBE_RESOLUTION, 180), 0.0);
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

        let target = PROBE_RESOLUTION;
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
        let target = PROBE_RESOLUTION;
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

        let target = PROBE_RESOLUTION;
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
