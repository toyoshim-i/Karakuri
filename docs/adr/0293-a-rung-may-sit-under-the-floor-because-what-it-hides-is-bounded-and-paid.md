---
id: 0293
title: A rung may sit under the floor, because what it hides is bounded and paid
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0095]
tags: [engine, estimate, renderer, measurement]
---

# A rung may sit under the floor, because what it hides is bounded and paid

## Context

[ADR-0285](0285-a-renderers-floor-is-bounded-from-its-declared-ranges-or-refused.md) landed this
morning and **closed the floor without making the estimate answer**. `karakuri_ir::rate` bounds a
renderer's `point_rate` from its declared ranges, so `estimate` has a floor where it used to have
`Unfit::FloorUnknown` — and every per-element Set in `examples/` moved from one refusal to another.
Of the fifteen shipped L4 procedures, three draw no primitive and were answered as they always were,
eight state a floor and **not one of the eight leaves room for two rungs at 720 rows**, and four
cannot be bounded at all. The rungs are half and a quarter of the target's height; the lowest floor
any per-element renderer states is 715 rows against an upper rung of 360. **Twelve of fifteen.**

ADR-0285 named the thing that was gating and left it:

> **How strictly the floor is read.** `estimate` enforces *no primitive anywhere in the frame may be
> rounded up*. For `soft_points` the primitive holding the floor at 4141 rows is a single element at
> rest. Whether a fit needs that, or only that the floored primitives' share of the coverage is
> negligible, is a question about `estimate`'s model that the bound does not decide.

**The maintainer's answer is to loosen it**, and his reason is
[ADR-0110](0110-this-machine-is-not-the-reference.md) reaching a second place:

> 床は緩くしろと言って気が。このマシンの100倍速いマシンでパフォーマンスするユーザーもいる

ADR-0110 is about reading this machine as evidence about every machine. *4 GiB of dedicated VRAM is
the figure a design is checked against, not a ceiling. Beyond that the performer's machine decides.*
The refusal here is the same shape one level up: **1280x720 is this repository's reference target,
the whole refusal is a comparison against it, and an operator on a machine a hundred times faster
is being handed no number at all on the strength of a size they are not using.** The instrument does
not add limits on behalf of hardware it does not have.

**What ADR-0285 got right and this record keeps:** the bound, the direction it rounds in, the
declarations travelling with it on `Estimate::floor_from`, and the refusal of a floor invented out of
nothing. What it got wrong is one clause — that a rung under the floor is a wall rather than a
quantity.

## Decision

**A rung may sit under the sub-pixel floor. What the flooring can then hide is bounded from the
placement alone, capped at a quarter, and divided back out of the answer.**

### 1. What a floored primitive costs the fit, exactly

Take one primitive that is `x` pixels across at the target, rungs at `H/p` and `H/q` rows, and
`λ = (A − A_lo) / (A_hi − A_lo)`, the factor `fit` extrapolates by. [ADR-0245](0245-the-sub-pixel-compensation-is-paid-in-the-colour-because-alpha-is-coverage.md)
makes its coverage `max((x·h/H)², 1)` at height `h`, so the coverage the fit predicts for it at the
target, less the coverage it really has there, is

```text
u(x) = 0                    x ≤ 1     floored at both rungs and at the target too
u(x) = x² − 1               1 ≤ x ≤ q floored at both rungs, not at the target
u(x) = (λ−1)(1 − x²/p²)     q ≤ x ≤ p floored at the lower rung only
u(x) = 0                    x ≥ p     floored nowhere
```

Two things fall out of it and they are the whole of the change.

**A primitive floored at the target as well costs the fit nothing.** `u(x) = 0` for `x ≤ 1`: it is
one pixel at both rungs *and* at the target, so it is a constant, and a fit that puts a constant in
`a` predicts it exactly. **This is the case ADR-0285's reading was refusing over.** The primitive
holding `soft_points` down to a 4141-row floor is 0.17 pixels across at a 720-row target. It was
never going to corrupt anything; the strict reading could not tell it apart from one that would.

**What a primitive can cost is bounded without knowing a thing about the rates.**
`u(x)/max(x², 1)` peaks at `x = q`, where it is `1 − 1/q²`, and a sum of ratios is bounded by the
largest of them. So **the share of the target's fragment cost the flooring can hide is at most
`1 − 1/q²`**, and `q` is the target's height over the **upper rung's** — nothing else enters. Not
the lower rung, not the distribution of rates, not the capacity, not whether the rate could be
bounded at all.

A stroke floors in one dimension rather than two and is milder throughout: at the placement below
its worst ratio is 0.056 against the sprite's 0.249. One rule answers for both, as
`sub_pixel_floor_rows` already does.

### 2. The threshold is a quarter, and it is derived twice

**From the bands the number feeds.** The console reads an estimate into five bands whose boundaries
are 4, 8, 12 and 16 ms — four slots to a 16.7 ms frame — and *a value on a boundary rounds to the
worse band* (`docs/manual/console.html`). The correction that makes a floored fit sound is
`1/(1 − share)`. At a share of a quarter that is `4/3`, and `4/3` moves a slot by at most one band
anywhere on that scale; the binding case is the last boundary, where a truth of 12 ms is reported as
16 — exactly the edge of the band that stops a slot. **A fifth more and the correction alone would
put a slot the truth leaves in red into purple**, and the number would be doing the badge's deciding
for it.

**From the probe's own bill, which is the same number.** `1 − 1/q²` is equally the share of a
full-size draw's fragment work the upper rung *declines to measure* — `q` is a height ratio and
coverage goes as its square. **The error a fit can hide is the work it did not do.** So this is not
a tolerance to be picked but a price, and it is paid where prices are: in the placement.

`FLOORED_SHARE_ALLOWED = 0.25`, in `crates/karakuri-engine/src/estimate.rs`, with both derivations
on it.

### 3. Where the rungs go

**The cheap pair, where the floor allows it** — half and a quarter of the target's height, the lower
raised to the floor. Unchanged from ADR-0266. Nothing is rounded up at either rung, the fit is exact
in ADR-0245's terms, `floored_share` is zero and no correction is applied.

**The accurate pair, where it does not.** The upper rung moves to `upper_rung_rows` —
`ceil(rows · √(1 − allowed))`, 624 at the reference target — and the lower one stays at a quarter,
where it is cheap and far enough from the upper rung to subtract against. That pair hides 0.2492 and
is corrected by 1.3319.

**Rounding up, not to nearest**: rounding down would put the share over the allowance the rung exists
to meet. And the allowance is checked **on the pair as placed**, not on the height that chose it —
`at_rows` truncates a width to an integer, so a rung can be a hair under the area its height implies,
which lifts `λ` and with it the share. `rungs` steps the upper rung up until the placed pair is inside
the allowance, and `floored_share` takes the greater of its two branches where they meet rather than
assuming they coincide. At the reference target neither correction moves anything: 624 rows hides
0.2492, where the height alone would say 0.2489.

**What it costs, and why that is not what it looks like.** The probe's fragment work goes from 0.31
of a full draw to 0.81. For the material that actually reaches this branch, measured, that is 0.81 of
nothing: `drift_shell + soft_points` cost 8.87 ms at 1280x720 and 9.00 ms at 640x360, a fragment term
of nothing at all, and what is paid twice either way is the invariant term. **It is a tendency and
not a rule** — a Set's floor is the *greatest* of its renderers', so one hairline renderer beside a
fullscreen one sends the whole Set down this branch. The compensation is the other half of the same
arithmetic: the answer is `(1−λ)·m_lo + λ·m_hi` with `λ = 1.361`, so noise in a rung is amplified
about 1.7-fold into the answer where the cheap pair amplifies it 9-fold.

### 4. The correction is applied to the answer, and it is what keeps the direction

`Fit::ms` is the fitted line divided by `1 − share`. The bound gives `truth ≤ fit / (1 − share)`
directly, and it is sound because `a ≥ 0` is already enforced. **It is applied to the sum rather than
to either term**: the undershoot is in the fragment work, but the fit misattributes part of it to
`a`, so the statement that holds is about the answer.

`Fit::invariant_ms`, `Fit::fragment_ms` and `Fit::ms_per_pixel` stay the raw line, so a caller can
see both the fit and the answer it supports — and where `Estimate::floored` is `None` the two are the
same number.

### 5. The record travels with it — P-0095

**An estimate taken with every primitive at least a pixel across at both rungs and one taken with
some of them rounded up are not the same statement.** `Estimate::floored: Option<Floored>` is that
distinction: `None` for the exact case, and otherwise the share and the correction it produced. It
sits beside `Floor`, which ADR-0285 added: **`floor_from` says where the floor came from, `floored`
says how strictly it was read**, and a consumer needs both to check the number. It is carried on
refusals too, because where the rungs sat is a fact about the attempt.

### 6. An unknown floor is the greatest floor there is, and no longer a refusal

The bound in §1 does not depend on the rates, so a renderer nothing can bound is covered by it
exactly as a bounded one is. `estimate` therefore passes `u32::MAX` to `rungs` where `floor_rows`
answered `Unfit::FloorUnknown` — a placement rather than a sentinel, since a floor of `u32::MAX` rows
says every primitive is under a pixel at any size anybody will draw, which is the worst case the
bound covers. `Estimate::floor` is `None` and says so. **The same for a contradicted bound**: a held
value outside the declaration a bound was taken over falsifies it, and a falsified bound is a floor
that is not known.

### 7. What still refuses

The loosening is not a licence, and four things still hand back no number.

- **A pair that would hide more than the allowance** — `Unfit::FlooringHidesTooMuch`, carrying the
  share and the allowance. `rungs` never produces one; a caller placing its own can, and the cheap
  pair against a 720-row floor is the worked example: `q = 2`, three quarters hidden. **This replaces
  `Unfit::RungBelowFloor`**, which was the flat form of the same question.
- **A target too short to hold two distinct rungs** — `Unfit::NoRoomBelowTheTarget` survives, now
  reachable only down the accurate branch where the upper rung rounds up onto the target itself:
  under eight rows.
- **Every fit refusal, untouched.** `FragmentTermNegative`, `InvariantTermNegative`,
  `InstrumentsDiffer`, `RungsCoincide`, `NotFinite`. A negative slope is still a failed measurement
  and not a cheap Set.
- **`Unfit::FloorUnknown` is still `floor_rows`' answer**, and is still what a caller asking that
  narrower question gets. What changed is that `estimate` no longer stops at it.

### 8. The reading of the bound: ADR-0282's rule, and what is missing

[ADR-0282](0282-a-rebuild-inherits-the-values-somebody-moved-and-reads-the-rest-from-the-code.md)
settled which state a number about a Set is taken over — **a declared value is the value in the
untouched state, and where somebody moved one the held value *is* the value** — and going stale when
a fader moves is what that reading is for, not a hazard against it: a write invalidates the estimate
standing on it and the estimate is taken again. `Set::rate_bound_contradicted` is the detection, and
ADR-0285's *the declared range and not the held value*, argued from staleness, is not the rule here.

**`karakuri_ir::rate::point_rate_bound_at` takes a bound that way** — params pinned to the values they
hold, declared ranges for the rest — and it is what the measurement below uses for its second column.
**What is missing is a caller.** `Set::rate_bounds` is computed once in `Set::build_many` over the
whole declared range, and following ADR-0282 needs the bound recomputed against `Set::params` — which
`Set` cannot do, because it does not keep the checked L4 procedures the walk reads. That is a change
in `crates/karakuri-engine/src/set.rs` and it is not this record's; `Set::moved` is not what it needs,
because `Set::params` **is** the state as it stands, holding the declaration where nobody moved it.

The measurement below is what says how much it is worth, and the answer is: after this record,
**exactness and probe time, and no longer an answer**.

## The measurement

Over the fifteen L4 procedures in `examples/`, at the reference target, taken with
`karakuri_engine::estimate::rungs` and `floored_share` — `mod corpus` in
`crates/karakuri-engine/tests/estimate.rs` is the same table mechanised, and
`crates/karakuri-ir/tests/rate.rs` is where the bounds themselves are written out.

| renderer | floor, declared | placed | floor, as it stands | placed |
|---|---|---|---|---|
| `field_lens`, `field_march`, `glow_march` | no primitive | clear | no primitive | clear |
| `plain_points` | 715 | 0.2492 | 240 | **clear** |
| `star_flares` | 715 | 0.2492 | 132 | **clear** |
| `sheet_shade` | 720 | 0.2492 | 720 | 0.2492 |
| `speed_lines` | 1450 | 0.2492 | 720 | 0.2492 |
| `soft_points`, `second_eye` | 4141 | 0.2492 | 514 | **0.1618** |
| `glass_shell` | 4141 | 0.2492 | 343 | **clear** |
| `drift_streaks` | 4141 | 0.2492 | 1711 | 0.2492 |
| `strand_strokes` | not known | 0.2492 | 164 | **clear** |
| `hard_dots`, `beat_strokes`, `beat_bloom` | not known | 0.2492 | not known | 0.2492 |

*Clear* is both rungs above the floor: nothing rounded up, the fit exact, `floored` is `None`. A share
is the accurate pair, corrected by `1/(1 − share)` — 1.3319 at 0.2492, 1.1930 at 0.1618.

**Before, under ADR-0285:** 3 answered, 12 refused — eight `NoRoomBelowTheTarget` and four
`FloorUnknown`, whichever way the bound was read.

**After, over the declared range:** 15 answered. 3 clear, 12 floored at 0.2492.

**After, over the state as it stands:** 15 answered. 7 clear, 8 floored — two of those at 0.1618,
their 514-row floor sitting between the rungs where the worst case cannot be reached.

So ADR-0285's *three of the eight would place rungs at 720 rows instead of none* survives as a fact —
`plain_points`, `star_flares` and `glass_shell` are exactly those three, and they are the ones that go
*clear* rather than *floored* — and it stops being the difference between a number and no number. The
narrower reading is now worth **four procedures' worth of exactness**, a third of the probe's fragment
work on each of them, and one procedure's worth of a bound at all: `strand_strokes`, whose `width_var`
is declared up to 1.0 where the rate reaches zero and is held at 0.45 where it does not.

**The rates the material actually emits**, measured, are what say the shipped corpus sits in the band
this is sized for. The last column is `x` from §1:

| material and setting | emitted rate | floor, rows | pixels at a 720-row target |
|---|---|---|---|
| `soft_points`, fastest | 0.00556 | 180 | 4.0 |
| `soft_points`, slowest | 0.00195 | 513 | 1.4 |
| `drift_streaks`, fastest | 0.00167 | 599 | 1.2 |
| `speed_lines` | 0.00139 | 720 | 1.0 |
| `drift_streaks`, slowest | 0.00058 | 1725 | 0.42 |

One row is under a pixel at the target and costs the fit nothing at all; the rest are between one and
four, which is the interval `u(x)` is non-zero on and the correction is sized for.

## Alternatives rejected

**Keep the strict reading and wait for a larger target.** It is already true that a 4K target places
the cheap pair for a 715-row floor, and it is the answer ADR-0110 forbids: the operator with the
faster machine gets a number and the one at 720p gets a refusal derived from a size neither of them
chose. It also does not reach `soft_points`, whose declared-range floor of 4141 rows would need an
8282-row target.

**Pick a share by feel and comment it.** Explicitly worse than the refusal it replaces. The number
here is fixed twice over — by the badge's own boundaries and by the probe's bill — and the two agree.

**Answer without the correction, and carry the possible 25% undershoot on the record.** It is
`P-0095`-shaped and it breaks *round the estimate toward refusing*, which is the one direction the
whole module is built to avoid. A consumer that ignored the record would act on a number that can be
a quarter low. The correction costs at most one band and makes the number safe to act on unread.

**Correct by the worst case always, rather than only where the floor is not clear.** `truth ≤ q²·fit`
holds unconditionally, so `×4/3` on every estimate would be sound. Rejected because where both rungs
clear the floor the fit is *exact*, and inflating an exact number by a third is inventing an error to
be safe from. The floor is what tells the two apart, which is what ADR-0285's analysis is for.

**Bound the share from the material rather than from the placement.** This is the reading ADR-0285's
open question literally asks for, and **it cannot be done.** A share needs a distribution over
elements; `point_rate_bound` gives one number, the smallest rate, and the worst case is that every
element sits at the peak of `u(x)/x²`. Publishing the upper end of the interval as well — the module
computes it and does not offer it — gives only a second all-or-nothing test (`r_max·H ≤ 1`: every
primitive floored everywhere, harmless) that nothing shipped satisfies. The placement bound is what
survives, and it is stronger for being distribution-free.

**Take a third rung and detect the flooring empirically.** Two points fit any line; three would show
the curvature the flooring puts in. It is a real measurement rather than a bound, and it costs a third
draw, a third `Probe::run`, and a residual test with a threshold on a noisy host clock — a threshold
this repository would then have to defend on every machine. The bound is free and does not have to be
re-taken.

**Raise the lower rung to the floor in the accurate branch too.** It would make the fit exact wherever
the floor sits under `upper_rung_rows`. Rejected because it converges the two rungs — a floor of 623
against an upper rung of 624 makes `b` a difference of two nearly equal numbers over almost no area —
and the flooring bound does not depend on the lower rung at all, so the accuracy is bought for nothing
and paid for in noise. **The same hazard already exists in the cheap branch**, where a floor of 359
raises the lower rung against an upper one of 360; it is untouched here and is named in *Consequences*.

**Move the upper rung further still and shrink the correction.** Every step toward the target buys
accuracy with fragment work, one for one, by §2 — which is the argument for stopping where the band
structure says the correction has stopped mattering. A share of 0.1 would put the upper rung at 684
rows of 720 and the probe's fragment work at 0.965 of a full draw against 0.814, to move a correction
from 1.331 to 1.108 — a fifth of a band, for a fifth again as much probe time, and a probe that has
stopped being a small draw.

## Consequences

- **`crates/karakuri-engine/src/estimate.rs` gains three public items** — `FLOORED_SHARE_ALLOWED`,
  `upper_rung_rows` and `floored_share` — all pure arithmetic, and `Floored` beside `Floor`.
  `Estimate::floored` is the new field.
- **`Unfit::RungBelowFloor` is gone and `Unfit::FlooringHidesTooMuch` replaces it.** A rung under the
  floor is a quantity now, and the variant carries both the share and the allowance.
- **`Fit::ms` is no longer `invariant_ms + fragment_ms`** where the rungs were floored. It is that sum
  times `Floored::correction`, and the doc on both fields says so.
- **`estimate` no longer returns `Unfit::FloorUnknown`.** `floor_rows` still does, and is still what a
  caller asking only about the floor gets.
- **`no_shipped_per_element_floor_leaves_room_under_the_reference_target` is gone.** It was written
  the morning ADR-0285 landed to mechanise the finding that the whole per-element corpus refused, and
  what replaced it is `every_shipped_floor_is_placed_and_none_hides_more_than_the_allowance` in the
  same file — the same four floors, asserting the placement and the share instead of the refusal —
  plus `mod corpus` in `tests/estimate.rs`, which is the finding itself re-measured over `examples/`
  under both readings.
- **The derivation is mechanised, not only argued.**
  `the_correction_covers_the_worst_frame_the_flooring_can_build` builds the adversary §1 claims to
  bound — a quarter of a million primitives all at `x = q`, coverage rounded up at each rung per
  ADR-0245 — fits it, and asserts the corrected answer is not below the truth while the raw fit is.
  `a_primitive_floored_at_the_target_too_is_predicted_exactly` is the `u(x) = 0` half.
- **`karakuri_ir::rate::point_rate_bound_at` is new** and additive: `point_rate_bound` is now it with
  an empty map. Nothing in the engine calls it yet — see §8 for what would have to change and where.
- **A pre-existing hazard is named and not fixed.** The cheap branch raises the lower rung to the
  floor with no minimum separation, so a floor just under the half-height rung produces two rungs of
  nearly equal area and a `b` that is a difference of noise. It predates this record, it is
  orthogonal to the floor's reading, and `Fit::ms_per_pixel` is where it would show.
- **`estimate::rungs` now searches upward for the upper rung** rather than computing it, bounded by
  the target's height, so that *within the allowance* is a property of the pair it returns rather
  than of the arithmetic that chose it. It takes the first candidate at every size in the corpus.
- **`estimate` still has no caller.** Wiring it to `Deck::govern` is untouched by this. What has
  changed is that there is now a number to wire for every piece of material this instrument ships.
