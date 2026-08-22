---
id: 0029
title: Amplification is a second kind of L2, not a new layer
status: accepted
date: 2026-07-26
supersedes: []
superseded_by: []
principles: []
tags: [ir, engine]
---

# Amplification is a second kind of L2, not a new layer

## Context

Raised as an intuition — position interpolation feels like a geometry shader, and once you think
that way, other things become possible. Kaleidoscopes, for instance.

The intuition names a real hole. **L2 is an endomorphism**, `Geometry -> Geometry`: it may change
an element's values but never their number. What a geometry shader has that L2 does not is
**amplification**, and so kaleidoscopes, instancing, trails and subdivision are inexpressible in
every layer this system has.

Three distinct capabilities were tangled in the suggestion, and separating them mattered:

| | | Status |
| --- | --- | --- |
| **Amplification** | one element becomes M | **Missing everywhere** |
| **Combination** | read the matching element of a second source | Missing; the count does not change |
| **Decimation** | N becomes fewer | Exists — `kill()` in L1, or a mask zeroing `size` |

Position interpolation is **combination, not amplification** — so an amplification layer would not
have solved it.

## Decision

Amplification is added as a **second kind of L2**, not a new layer number. L0–L5 already carry
meanings and inserting between them shifts everything; both kinds take geometry and return
geometry, so they occupy the same slot position. What differs is the **count mode** — a term the
`Geometry` type declaration already used.

It fits the system's bet: the roadmap argues that multiple L4s over one geometry are the cheapest
visual variety per GPU-second, and **amplification is the same argument on the geometry side.** One
simulated element producing eight mirror images means one simulation and eight times the drawing,
which is an order cheaper than simulating eight.

**And it is cheaper to build than L1 was.** The amplified output is a derived buffer rebuilt from
its input every frame, so it holds no state: **no double buffering** (there is no previous value to
read) and **no compaction** (liveness is decided on the input side).

## Consequences

- The rate must be **declared** — `amplify 8` — because stacking multiplies (N → 4N → 16N). A
  compile-time constant sizes the output buffer and keeps cost estimation a multiplication, which
  is the same "only what is statically bounded" rule as `capacity` and loop bounds.
- **Identity returns in a new shape.** Eight copies of one element must be distinguishable, so
  identity becomes the parent's `seed` paired with a **copy number** readable only in that stage —
  the same kind of implicit value as the `source` of
  [ADR-0024](0024-each-source-counts-from-zero-and-carries-a-source-attribute.md).

## Evidence

Session 2026-07-26T07:40Z–07:42Z. Scheduled into M3 and marked unimplemented under the
specification's `Beyond v0.2` boundary.
