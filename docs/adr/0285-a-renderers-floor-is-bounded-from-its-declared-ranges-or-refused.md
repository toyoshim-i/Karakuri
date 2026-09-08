---
id: 0285
title: A renderer's floor is bounded from its declared ranges, or refused
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0095]
tags: [ir, engine, renderer, measurement]
---

# A renderer's floor is bounded from its declared ranges, or refused

> **Annotated 2026-09-08, later the same day: one clause is loosened, and it is the one this record
> named as open.**
> [ADR-0293](0293-a-rung-may-sit-under-the-floor-because-what-it-hides-is-bounded-and-paid.md)
> decides that **a rung may sit under the sub-pixel floor**, because what the flooring can then hide
> is bounded from the placement alone — `1 - 1/q²`, where `q` is the target's height over the upper
> rung's — capped at a quarter and divided back out of the answer. That is *What is open is the
> reading of the floor* below, answered: **less strictly than `estimate` was enforcing it**. A
> primitive floored at the target too costs the fit nothing, and the strict reading could not tell
> it apart from one that would. `Unfit::RungBelowFloor` is gone and `Unfit::FlooringHidesTooMuch`
> replaces it; `estimate` no longer returns `Unfit::FloorUnknown`, because an unbounded rate is the
> worst case the placement bound already covers and is therefore placed rather than refused.
>
> **This record stands whole and is not superseded** — ADR-0293's `supersedes` is empty on purpose.
> The bound, the direction every rule rounds in, the declarations travelling with the floor on
> `Estimate::floor_from`, and the refusal of a floor invented out of nothing are all kept, and are
> what ADR-0293 is built on. What moved is one clause: that a rung under the floor is a wall rather
> than a quantity. The finding below — *none of the eight leaves room for two rungs under a 720-row
> target*, twelve of fifteen refused — survives as a fact and stops being the difference between a
> number and no number: all fifteen are answered now, three of them with both rungs clear of the
> floor. The test written that morning to mechanise the refusal,
> `no_shipped_per_element_floor_leaves_room_under_the_reference_target`, is replaced by
> `every_shipped_floor_is_placed_and_none_hides_more_than_the_allowance` over the same four floors.

## Context

[ADR-0245](0245-the-sub-pixel-compensation-is-paid-in-the-colour-because-alpha-is-coverage.md)
puts a floor under any draw of a Set that emits a primitive: a primitive of side `s < 1` pixel is
drawn at one pixel and dimmed, so **below `1 / rate` rows its coverage stops falling and becomes
one**. `karakuri-engine`'s `estimate` fits `a + b·area` through two small draws, and a rung under
that height measures a different picture — it reads high, drives `b` down, and makes the estimate
**undershoot**, which is the one direction *round the estimate toward refusing* forbids.

`point_rate` is a per-element vertex expression. Nothing on the CPU evaluates it and nothing can
enumerate the elements it would be evaluated over, so `estimate` answered `Unfit::FloorUnknown` for
every `Points` and `Lines` Set and drew nothing. It offered `estimate_above_floor` to a caller who
knew the rate, and **no caller knew it**: the number is in the `.kir` file and nowhere else.

What the file does carry is a **declared range on every param** — `param p : float [min, max] =
default` — which `crate::ast::Param` describes as the fader's range, the agent's search range and
the normalisation basis for signal binding at once. That is enough to bound the expression without
evaluating it.

## Decision

**`karakuri_ir::rate::point_rate_bound` bounds a renderer's `point_rate` from below by interval
arithmetic over its params' declared ranges, and `estimate` takes its floor from that. Where it
cannot bound the rate above zero it answers `Unfit::FloorUnknown` exactly as before.**

Three things are load-bearing.

**It is a *lower* bound, and every rule rounds outward.** The floor is `1 / rate`, so a bound below
the true smallest rate gives a floor *above* the true floor — rungs placed higher than they had to
be, and at worst `Unfit::NoRoomBelowTheTarget`. A bound *above* it gives a floor *below* the true
one, which certifies a draw whose primitives are being rounded up: the undershoot ADR-0245's
consumer forbids. Every operation in the module is therefore sound-in-that-direction rather than
tight: a leaf with no range is the whole real line, `p - p` is `[min - max, max - min]` rather than
zero, and an operation that produces `NaN` degrades to "not known".

**The bound is over the *declared* range, never the value the Set is holding.** An estimate is taken
once while a slot primes and read for as long as the slot is on air, and a param moves under a
fader, a signal or an agent for that whole time. A floor read off the held value would be right at
the instant it was taken and wrong the first time anybody turned a knob — and nothing would say so,
because the badge would still be showing the number. The declared range is a property of the *file*,
so a floor taken from it is true for as long as that file is the material.

**Where it cannot bound, it refuses.** Two failures reach the same answer and are distinguishable on
the record: an expression nothing bounds below (`dot_scale * size`, where `size` is an attribute),
and one that genuinely reaches zero (`point_scale * (1.0 - width_var + …)` with `width_var` declared
up to 1.0). The second is not a defect in the analysis — a primitive that can be zero across has no
height at which it is a pixel — and a floor invented for either is the unsafe direction by
construction.

**Nothing clamps a write to a declared range**, which `Set::carry_moved_from` says outright of a
carried value and of `--param` alike. So a bound is a claim about the file that a held value can
falsify, and `Set::rate_bound_contradicted` is the check: for the params a bound actually named, is
the value the Set is holding inside the declaration it was taken over? A contradicted bound is not
used — `Unfit::FloorUnknown`, with the offending name, value and declaration on the record.

**The record travels with the floor**, which is
[P-0095](../principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md)
one level up from the measurement: `Estimate::floor_from` carries a `Floor::Analysed` with one
`RateBound` per renderer — the procedure's name, the rate proved, and **the declarations the proof
rests on**. A caller that cannot see those cannot check the floor, and a floor nobody can check is
the same shape of number `MeasurementMethod` exists to prevent.

## What it closes and what it does not

Over the fifteen L4 procedures in `examples/`: three draw no primitive, **eight state a floor**, and
four are refused. The eight are written out in `crates/karakuri-ir/tests/rate.rs`.

**None of the eight leaves room for two rungs under a 720-row target.** The rungs are half and a
quarter of the target's height, so a floor above 360 rows has nowhere to stand, and the lowest floor
any of them states is 715. So `estimate` moves from `Unfit::FloorUnknown` to
`Unfit::NoRoomBelowTheTarget` for the shipped corpus — **a different refusal, carrying a number, and
one that says what would have to change** — rather than to an answer.

That was already visible and was read as a fact about one Set. `estimate`'s own module documentation
carried five measured rates before this record, of which one — `speed_lines` at 0.00139, "its floor
is the reference size itself" — was named as the case with no room. **Three of the other four are
also above 360 rows.** The exception is not `speed_lines`; the exception is `soft_points` at its
default `point_scale` with every element moving, and the same procedure emits a 513-row rate for any
element at rest in the same frame.

Three of those five floors were also arithmetically wrong — `1 / 0.00139` is 719.4 and the table
said 719, against a `sub_pixel_floor_rows` test in the same file asserting 720. They are recomputed
where they stand.

**What is open is the reading of the floor, and it is not this record's.** As `estimate` enforces
it, the condition is *no primitive anywhere in the frame may be rounded up*, and for `soft_points`
the primitive holding it down is a single element at rest emitting 0.35 of the param. Whether a fit
needs that, or needs only that the floored primitives' share of the coverage is negligible, is a
question about `estimate`'s model that this bound does not decide either way — it reports the
smallest rate, and how strictly that is read is above it.

## Alternatives rejected

**Bound over the values the Set is holding rather than over the declarations.** Tighter by a lot:
three of the eight would place rungs at 720 rows instead of none. Rejected because the number goes
stale silently — the estimate is taken at priming, the badge is read for as long as the slot is on
air, and a fader moved afterwards invalidates the floor with nothing anywhere saying so. It is the
same failure `Probe::run` demoting for life exists to prevent, one level up: a number whose basis
moved after it was taken. **The two are not exclusive** — `Bound::AtLeast::over` names the
declarations, so a caller holding the values has everything it needs to compute the tighter floor
and own the staleness — and this record takes the one that is true for the run.

**Corrected 2026-09-08: *silently* is wrong, and the rule it argues against was already settled when
this was written.**
[ADR-0282](0282-a-rebuild-inherits-the-values-somebody-moved-and-reads-the-rest-from-the-code.md)
landed at 11:32 that morning and this record at 13:20. It settles which state a number about a Set is
taken over — a declared value is the value in the untouched state, and where somebody moved one **the
held value *is* the value** — and it is also what makes the movement visible: `Set::moved` marks
every value `Set::set_param` or `Set::set_param_at` wrote, per node, and those two are the only
places a number in `Set::params` ever changes. So a fader moved after a held-value bound was taken is
not moved *with nothing anywhere saying so*; the Set says which of its values somebody stated, and
`Set::rate_bound_contradicted` above is a second thing that says so where the value also leaves its
declaration. Under ADR-0282's reading the staleness is the design rather than a hazard against it — a
write invalidates the estimate standing on it and the estimate is taken again — which is
[ADR-0293](0293-a-rung-may-sit-under-the-floor-because-what-it-hides-is-bounded-and-paid.md) §8, where
this alternative is re-read and `karakuri_ir::rate::point_rate_bound_at` is the bound taken that way.
**The decision is untouched by the correction**: `Set::rate_bounds` is still computed once over the
declared ranges, nothing in the engine calls the held-value form, and what the tighter reading is
worth is measured in ADR-0293 — four procedures' worth of exactness and one bound at all — rather
than assumed here.

**Evaluate the expression on the GPU and read the minimum back.** It is the exact answer rather than
a bound, and it costs a pass, a readback and a synchronisation per estimate, and it is a measurement
of *this frame's* elements rather than a property of the material — so it goes stale on the next
step, not merely on the next fader move. The bound is free, static, and does not have to be redone.

**Widen the analysis until the four refusals answer.** Two of the four cannot be answered by any
analysis: the rate really does reach zero. The other two read attributes, and bounding an attribute
means bounding the simulation that wrote it — a whole-chain analysis for two files, where the
author's own fix is a `max(size, 1.0)`, which is what `star_flares` already writes and what
`hard_dots` does not.

**Keep `Topology` as the discriminator and treat `Points`/`Lines` as unknowable.** That is the state
this replaces. It was honest, and it made the estimate answer for nothing per-element at all — so
the badge had no band to draw for any of the material this instrument ships, and there was no
arithmetic anywhere saying why.

**Claim a range for `age`, `t` or the noise builtins.** `age` is non-negative in every use anyone
has written and nothing in the language enforces it — an L2 `deform` block assigns attributes — and
`perlin`, `simplex`, `fbm` and `curl` are bounded in practice with the specification silent on by
what. A bound this file invented would be a claim no other file is holding to, and it would be
holding up a floor. `hash1` and `value_noise` **are** claimed, from `docs/ir-spec.md`'s built-in
table, which states them.

## Consequences

- **`crates/karakuri-ir/src/rate.rs` is new** and is the whole of the analysis: an interval, the
  operations on it, the builtin table, and a walk over the checked vertex block. Pure — no device,
  no Set, no held values — which is what makes it answerable before anything is drawn.
- **`estimate::floor_rows` takes bounds rather than topologies** and returns the greatest of the
  per-renderer floors, or `Unfit::FloorUnknown`. `Topology` now decides **nothing** in
  `karakuri-engine::estimate`; it is reported on `Estimate::topologies` so a number can be read
  against what was drawn, and that is all.
- **`Estimate` carries `floor_from`.** `Floor::Stated` for `estimate_above_floor`, which is the
  record that nothing stood between the caller and ADR-0245's failure mode, and `Floor::Analysed`
  otherwise.
- **`Set` computes the bounds once at build**, one per L4 procedure in `L4:n` order, from the same
  walk `params` and `ranges` come from — so a bound and the declarations it was taken over cannot
  end up at different indices.
- **A test that used to assert `Unfit::FloorUnknown` for a per-element Set now asserts a draw.**
  `crates/karakuri-engine/tests/estimate.rs` keeps the refusal on material that earns it — an
  attribute-derived rate, and a param written below its declaration.
- **`estimate` still has no caller.** Wiring it to `Deck::govern` is untouched by this, and the
  precondition it was waiting on is now met in the sense that a floor exists and **not** in the
  sense that the shipped material can be measured at the reference size.
