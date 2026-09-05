---
id: 0252
title: The language is bounded, so a price can be computed before anything is built
status: accepted
date: 2026-09-05
supersedes: []
superseded_by: []
principles: [0091]
tags: [ir, engine]
---

# The language is bounded, so a price can be computed before anything is built

## Context

**This decision predates the recovered history and was never written down.** It was carried by
P-0067 — *The language is bounded, so a procedure can be priced before it runs* — whose retirement
into
[P-0091](../principles/0091-cost-is-known-before-it-is-paid.md) is what surfaced the gap: the
directory's own tombstone list said the reasoning was *"in `ir-spec.md` and the check pass"*, which
is where the rule is stated and not where its alternative lost.

The nearest records presuppose it rather than decide it.
[ADR-0004](0004-var-is-added-so-a-loop-can-carry-a-value.md) adds `var` so a `for` body can carry a
value, and says of cost that *"the estimate stays instruction count × loop bound"* — a sentence that
is only meaningful if the bound is a literal.
[ADR-0013](0013-cost-has-three-axes-that-must-not-be-added.md) decides what the axes are and that
they must not be added, and takes for granted that there is a number on each of them. Neither says
why a procedure may not write `while`.

## Decision

**Every loop bound in a `.kir` is a signed integer literal. There is no `while` and no recursion.**
Nested loops multiply into the estimate, and a range that does not ascend — `for i in 4..0` — runs
zero times and is charged for zero times.

**What that buys is the whole safety story**, and it is the reason the constraint sits in the
grammar rather than in a lint. A procedure's cost is computed by the check pass, before anything is
compiled and before anything runs, which is what lets an expensive mistake be **refused** rather
than survived. Everything downstream rests on that number existing: the frame budget a swapped-in
Set is held to, the rollback that returns the previous procedure when a candidate is over budget,
and the promise that a model can write badly without taking the show down.

**The refusal states the constraint rather than the correction**, because the correction depends on
what the author was trying to do: *"loop bounds cannot reference a param, an ambient, or any other
expression — cost estimation needs them fixed at parse time."*

## Alternatives rejected

**A bound that reads a param, so a procedure can adapt itself.** The one that keeps being reached
for, and the one the hint above exists for. It makes the trip count a function of a value that is
not known until the Set is composed and can change under a running slot, so the estimate stops being
a property of the artifact — which is where stages 1–5 of the pipeline put it precisely because they
are capacity-independent.

**`while` for an iterative solver, and recursion for a tree walk.** The other two, and both are the
same defect in different clothes: a trip count that is not knowable at parse time turns a price into
a guess.

**Estimate optimistically and rely on the runtime rollback.** The alternative that is proposed
alongside all three, and the one that sounds cheapest because the rollback already exists. It moves
a refusal that costs nothing — a compile that fails, before anybody is watching — into one that
costs a dropped frame in front of an audience, and it does so **only for the procedures whose cost
was never knowable in the first place**, which is the set of procedures least likely to be cheap. A
governor that cannot price a candidate has nothing to do but run it and watch.

## Consequences

- **A refusal is available at every stage that has a number, and the earliest one is free.** That
  ordering — refuse where refusing costs nothing, roll back where it costs a swap somebody asked
  for, be loud where neither is available — is argued from this domain outward in
  [ADR-0234](0234-carrying-a-show-through-is-a-principle-not-a-property-of-the-finished-instrument.md).
- **The estimate may be conservative and stage 7 is the authority.** `ir-spec.md` says so: the
  weights are ordinal inventions, and the estimate exists to reject the obviously impossible
  cheaply, not to predict a millisecond.
- **The bound is a parse-time property, so nothing later can widen it.** `parse.rs` accepts a signed
  integer literal in a `for` bound and produces a diagnostic for anything else, which is why no pass
  after it has to reason about trip counts.
