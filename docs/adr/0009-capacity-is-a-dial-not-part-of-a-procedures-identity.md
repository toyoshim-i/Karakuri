---
id: 0009
title: `capacity` is a dial, not part of a procedure's identity
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: [0087]
tags: [ir, engine, store]
---

# `capacity` is a dial, not part of a procedure's identity

## Context

`capacity` was a compile-time constant in the artifact, listed as a constraint that might be
relaxed later.

## Decision

Not a constraint to keep — **a design error to fix**. `capacity` is a runtime performance
dial, not part of what a procedure *is*. Left in the artifact, the library multiplies by every
size anyone ever wanted: the 65536 and the 262144 version of one procedure get different
hashes and different previews, which is wrong for a library of material.

The `.kir` declares a **range with a default** (`capacity [65536, 1048576] = 262144`), the Set
overrides it, and the ambient becomes a **uniform** rather than a constant — so changing it
costs a fork and a swap but **no recompilation**.

## Alternatives rejected

- **Keep it compile-time and relax later.** Accepts artifact multiplication in the meantime,
  and the library is the asset.
- **Drop it from the artifact entirely.** The procedure does know what sizes it is sane at,
  which is why a declared range replaced a declared value.

## Consequences

- **Cost estimation gets cleaner.** An artifact records cost *per element*, which is the
  intrinsic figure; any capacity's cost is a multiplication.
- Validation stage 4 changes shape: stages 1–5 are per artifact and capacity-independent (the
  WGSL is identical at every capacity), stages 6–8 are per Set, and **the budget judgement
  moves to Set construction**.
- The per-element figure is honest only for L1. L4 is fill-rate dominated, so `perf` takes two
  forms — see [ADR-0013](0013-cost-has-three-axes-that-must-not-be-added.md).
- **`capacity` is also a visual parameter** in a procedure with no `spawn` block, because
  `seed` equals the initial slot index there and `seed % 512u` builds a lattice. Guidance:
  derive structure from the ambient, `uint(sqrt(float(capacity)))`.

## Evidence

Session 2026-07-25T12:03Z–12:24Z. Standing rule:
[P-0087](../principles/0087-name-the-property-never-the-shape.md).
