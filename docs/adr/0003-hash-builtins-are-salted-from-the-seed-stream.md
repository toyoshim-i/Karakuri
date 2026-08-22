---
id: 0003
title: Hash builtins are salted from the layer's seed stream
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []   # a later record moves the salt per source; number assigned when it is written
principles: [0008]
tags: [ir, determinism]
---

# Hash builtins are salted from the layer's seed stream

## Context

Naming the per-element ordinal `seed` collided with a record the Set format already had:
`{"t":"seed","stream":"L1","value":19274}`. If the element value is `seed` and the stream
value is also `seed`, re-seeding a procedure to get a different look has no defined effect.

## Decision

The element value keeps the name `seed`, and the **hash builtins are salted** with the
layer's seed stream value instead.

The consequence is the reason: `hash1(seed)` changes when the layer is re-seeded, and
`seed % 512u` does not. **Structure and randomness become independently controllable** —
re-seeding a Set changes its dust without moving its lattice.

## Alternatives rejected

- **Mix the stream value into `seed` itself** and present one word. Simpler to describe, and
  it moves the lattice when you re-seed, which is exactly the control being sought.
- **Rename the element value** (`spawn_ordinal` was proposed). Costs the shorter name at
  every use site for a collision that salting already resolves.

## Consequences

- A hash is **no longer a pure function of its argument** across Sets. Stated explicitly in
  the builtins section, because it is surprising and would otherwise be discovered.
- Satisfies the standing invariant that randomness comes only from an explicit seed stream.

## Status note

This holds **per layer**, and stops being the whole truth once a Set can hold more than one
geometry: two geometries built from the same procedure would draw the same dust. The salt
moves to **per source** on 2026-08-16, and this record is superseded there — the front
matter is updated when that record is written.

## Evidence

Session 2026-07-25T11:50Z. Standing rule:
[P-0008](../principles/0008-structure-and-randomness-are-independently-seeded.md).
