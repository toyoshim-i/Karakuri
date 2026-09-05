---
id: 0005
title: Spawn is an accumulator, and the engine owns the birth fraction
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: [0090]
tags: [ir, engine, determinism]
---

# Spawn is an accumulator, and the engine owns the birth fraction

## Context

How many elements a frame creates was open between a deterministic accumulator and a Poisson
process. The question was framed as regularity of the count at low rates.

## Decision

**Accumulator, and no Poisson.** If irregularity is wanted it belongs on a noise signal bound
to `spawn_rate`, where its depth and its period stay adjustable; baked into the engine it
becomes a fixed property nobody can turn. This follows the standing preference for expressing
in a binding what a binding can express.

The framing was also wrong, and correcting it is the more useful half of this record. What
actually breaks a particle stream at low rates is **spatial banding**, not count regularity:
every element born in one frame starts at the same phase, so discrete shells travel outward.
Poisson does not fix that.

The fix is a **birth fraction owned by the engine**. Each element carries a fraction, and its
**first `element` pass runs with `dt` scaled by it**. Nothing about this appears in the IR.

The fraction is pinned to `(float(j) + 0.5) / float(count)` — equally spaced rather than
carry-exact — because a replay must agree, and "whatever fraction the engine passed" does not
reproduce.

## Alternatives rejected

- **Poisson in the engine.** Fixes nothing that is visible and removes a control.
- **An ambient `spawn_frac` the procedure applies itself.** The engine cannot advance a
  generic integration, so the correction would have to be written by the author — and it
  would be *forgotten by half the generators that need it and applied twice by the other
  half*, with a silent failure either way. The engine owning it makes banding structurally
  impossible whatever a procedure writes, and adds nothing to the generation prompt.
  Interpolating a spawn position along a moving emitter needs the ambient version, but an
  emitter position is a param and constant within a frame, so v0.2 cannot express it anyway.

## Consequences

- `dt`'s description as a fixed simulation step gains "except an element's first update".
- Implemented without a branch: `element` computes `_dt = u.dt * birth_frac_prev` and writes
  `birth_frac_next = 1.0`, so the correction expires itself. This was the code generator's
  improvement on the specification, which had called for a first-update test.

## Evidence

Session 2026-07-25T12:03Z–12:24Z; the branchless lowering at 2026-07-25T14:24Z. Standing
rule: [P-0090](../principles/0090-a-surface-offers-it-never-decides.md).
