---
id: 0002
title: Compaction preserves order, and determinism means bit-exact
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: [0092]
tags: [engine, determinism]
---

# Compaction preserves order, and determinism means bit-exact

## Context

Removing `id` ([ADR-0001](0001-identity-is-seed-an-ordinal-not-a-slot-index.md)) settled
that elements move. How they move was still open, and the first argument offered for
order-preserving prefix-sum compaction was determinism: GPU atomics complete in a
non-deterministic order, so an atomic free list would not reproduce.

That argument is true and it is not the strongest one available, which matters, because a
specification resting on its second-best reason is easier to argue out of later.

## Decision

Compaction is an **order-preserving prefix-sum stream compaction**, and the primary
argument is **indirect dispatch requires contiguity**. Dispatching `element` over the live
count means live elements must be contiguous in the buffer. A free list leaves them
scattered, so dispatching over a live count would need a separate packed array of live slot
indices — and building that is a scan. The free list does not escape the scan; it only
declines to move the elements while paying for it. Having paid, moving them also aligns
memory access.

Determinism is then a **consequence, not a premise**: choosing prefix sum gets order
preservation, and order preservation gets bit-exact reproduction for free.

Separately, `README`'s "the same output" is resolved to **bit-exact**. Once `id` is gone,
the only path observing element order is blend composition — and floating-point addition is
not associative, so a changed order changes the low bits even under additive blending.
Bit-exact makes order preservation load-bearing; behaviour-exact would not have, and
non-order-preserving compaction would then have been legal.

## Alternatives rejected

- **Atomic free list.** Cheaper per frame, and forfeits both properties. It also does not
  avoid the scan, which is what makes the trade one-sided rather than a trade.
- **Behaviour-exact determinism** (same look and same parameter response, low bits free).
  Would have permitted the cheaper compaction. Rejected because a replay that is only
  approximately the performance is not a replay.

## Consequences

- A scan pass proportional to the live count runs every frame, which is a performance-touching
  change and so required measurement at full capacity in the first implementation slice.
- The scan fuses into the `element` pass; the free region is the contiguous range
  `[live_count, capacity)`; there is no free list anywhere in the system.
- Draw order is oldest-first as a by-product, which stabilises the look of non-commutative
  blend modes run to run.

## Evidence

Session 2026-07-25T11:44Z. Implemented as a recursive hierarchical scan, depth
`ceil(log64(capacity))`, verified against a CPU reference at 64 / 65 / 4096 / 262144
elements (2026-07-25T15:00Z). Standing rule:
[P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md).
