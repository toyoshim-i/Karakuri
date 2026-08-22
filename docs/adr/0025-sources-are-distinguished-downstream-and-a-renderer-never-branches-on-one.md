---
id: 0025
title: Sources are distinguished downstream, and a renderer never branches on one
status: accepted
date: 2026-07-26
supersedes: []
superseded_by: []
principles: [0021]
tags: [ir, render]
---

# Sources are distinguished downstream, and a renderer never branches on one

## Context

Multiple sources had been discussed entirely as *blending* — how to mix A and B. The more basic
use had been missed: keeping them apart. "Draw L1A red, L1B green, L1C blue." "Scale only L1A's
coordinates with the volume."

## Decision

**Distinguishing is the main path; blending is the special case.** The main path needs no new
machinery — `source` plus the attribute masks L2 already has. "Only L1A, scaled by volume" is an
L2 modulator masked on `source == 0` with the volume signal bound to its parameter.

**And an L4 never branches on `source`.** L2 writes `tint` through a mask; L4 draws `tint`. The
renderer then **does not know how many sources there are**, so one written for a single source
works unchanged with five. Once `if source == 0` appears in an L4, that L4 depends on the Set's
source arrangement and stops being reusable.

Blending has three readings, and two need nothing new:

| | What it is | Cost |
| --- | --- | --- |
| Crossfade | Draw both, mix opacity | **Nothing new** — it is L5's job, done as two Sets rather than two sources, so no L4 branches |
| Dissolve | Hide one source's elements progressively | **Nothing new** — a mask on `source` and `hash1(seed) > mix`, zeroing `size` or `tint` |
| Position interpolation | Pair by `seed` and lerp | **A real addition** — a cross-element read |

Position interpolation is the only one that is genuinely new, because the IR is built on each
element reading only its own attributes. It also collides with compaction: A and B are killed
independently, so `seed 5` sits at different slots on each side and pairing needs a reverse
index. **Restricted to sources with no `spawn` block**, where `seed` is the slot index and
compaction never runs, it is just reading the same index — and lattices and shells are almost all
of that class.

## Evidence

Session 2026-07-26T07:32Z–07:39Z. Standing rule:
[P-0021](../principles/0021-a-renderer-does-not-know-how-many-sources-there-are.md).
