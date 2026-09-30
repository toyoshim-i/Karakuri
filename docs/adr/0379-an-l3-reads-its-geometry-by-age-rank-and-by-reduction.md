---
id: 0379
title: An L3 reads its geometry by age rank and by reduction
status: accepted
date: 2026-09-30
supersedes: []
superseded_by: []
principles: [0086, 0091, 0092]
tags: [ir, engine, codegen, camera, l3]
---

# An L3 reads its geometry by age rank and by reduction

## Context

[ADR-0095](0095-a-camera-edge-is-a-gpu-buffer-and-an-l3-may-hold-state.md) made the Camera edge a
GPU buffer so that a camera could follow geometry without a readback.
[ADR-0099](0099-element-zero-is-the-oldest-living-element.md) decided what it may follow:
reductions (centroid, bounds) and element 0, the oldest living element. Neither was built. The
specification listed a third candidate, the element with `seed == N`, as not built because there
is no seed-to-index map and finding one is a search over `capacity`.

The request that built it was a camera flown by two points of its own geometry: stand on one
vertex, look at another. Element 0 alone cannot do that; the target would have to be computed
from something else.

## Decision

**An L3 may declare one `uses <name> : Geometry` slot and reads it in three ways:**
`<name>[i].<attr>`, `<name>.centroid`, and `<name>.bounds_min` / `<name>.bounds_max`. The slot
is bound by an `edge` to an L1 that heads a chain.

**`i` is an age rank into the live range, not a seed.** Compaction is order preserving, so the
argument ADR-0099 made for index 0 holds for every index: index `i` is the `i`-th element after
the oldest living one, and reading it is one buffer load. This is the cheap half of the rejected
`seed == N` row without the search. Where a source has no `spawn` and no `kill()`, age rank and
seed coincide, which is what lets an L1 place camera points at its first seeds. An index past the
range reads its last entry.

**`consumes` on an L3 names what it reads through the slot**, as an L2's `consumes` covers
`far.<attr>`. The reductions require `position`. The Set refuses a camera whose source neither
emits nor synthesises what it consumes ([P-0086](../principles/0086-a-procedure-knows-only-what-it-declares.md)).

**The reduction is one extra entry point in the L3's module**, dispatched before the camera
block only when the block reads a reduction. It skips elements killed in the step. Its cost is
charged in the L3's estimate per element of the source
([P-0091](../principles/0091-cost-is-known-before-it-is-paid.md)).

**An empty live range skips the camera block**, so the state holds, as ADR-0099 required. To
make "holds" mean something before the source ever has an element, a camera that reads geometry
is primed once with the built-in orbit's placement.

**The Set builds its sources before its cameras**, because a camera's bind groups name its
source's element buffers, and the camera pass takes the source's parity at record time.

## Alternatives rejected

- **Element 0 only, as ADR-0099 wrote it.** Smallest change, and it cannot express a camera
  aimed from one element at another.
- **`seed == N` addressing.** Needs a seed-to-index map maintained through compaction, for a
  gain only in spawning sources, where a seed's element may already be dead.
- **Reading any emitted attribute without `consumes`.** Moves a check the procedure can make on
  its own header into Set build, and makes an L3's reads invisible in its interface.
- **Index syntax as a builtin call (`element(subject, i)`).** `[ ]` reads as what it is, and the
  checker refuses it everywhere except after an L3's geometry slot, so the language gains no
  arrays.

## Consequences

- **An element killed in the step is still readable by index** until the next step compacts it
  away. The reductions exclude it; an index read does not check the alive flag.
- **A camera reads one geometry.** Reading a second source, and binding to the far side of a
  pairing, are refused.
- `examples/ribbon_chase.kir` places its eye and target at elements 0 and 1, and
  `examples/ribbon_eye.kir` flies them. `examples/centroid_tracker.kir`, which
  [ADR-0374](0374-the-built-in-camera-expands-to-six-placement-parameters-and-procedural-l3-camera-procedures.md)
  shipped with a stand-in centroid computed from `t`, now reads `subject.centroid`.
