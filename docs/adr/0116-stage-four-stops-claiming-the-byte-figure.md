---
id: 0116
title: Stage 4 stops claiming the byte figure; the engine reports it
status: accepted
date: 2026-08-20
supersedes: []
superseded_by: []
principles: [0087]
tags: [ir, engine]
---

# Stage 4 stops claiming the byte figure; the engine reports it

## Context

Unifying the placement rules moved `generate_element_layout` into the IR so the estimator could
charge what the engine allocates. A review then measured what the estimator actually produced.

**Every L2 in the repository was under-reported by 69–83%.** The estimator built the layout from that
procedure's own `emit`; the engine sizes the buffer from the **widened** list — upstream's emits plus
its own. And **all four L2 examples declare no `emit` at all**, so the estimator's stride was
`seed + birth_frac (+ copy)`: 96 bytes reported against 312 allocated, 25.2 MB against 81.8 MB on one
node at default capacity.

**The dangerous part is not the number.** The old code was **documented as broken** — *wrong in both
directions and nothing reads it*. The new code was **documented as exact**: *the layout itself, not a
model of one*. And an under-report is the dangerous direction for M4's metadata: a residency check
passes and the allocation fails.

## Decision

**Stage 4 stops producing the figure, and the engine reports it.**

The three inputs it needs — the widened emit list, the amplification factor, which derivations are
stored — are only settled *after* stage 4. The engine's numbers come from `wgpu::Buffer::size()` of
buffers the node created: **there is no second expression to drift.**

## Consequences

- **An agent refused an instruction of mine, correctly.** I said to include the `counts` buffer in the
  total; it declined, because mixing fixed-size scratch in means uniforms, step args and L4 render
  targets must come too — or the number **claims to be a residency total while missing most of one.**
  Better judgement than mine.
- `compaction.rs` allocates more than `dest` — a pyramid of block sums — but **sublinear in capacity**,
  so it cannot enter a per-element figure. The line is drawn at *one entry per element* and the
  exclusion is written into `ElementStorage`.
- **The naga test reads naga's own offsets, span and stride**, not the arithmetic. Nothing emits
  `@offset`, so a front end handed a wrong table **agrees with itself** — validation alone could never
  catch it.
- A methodology find: the first three injections failed at `validate()` with a parse error rather than
  at the offset comparison, and were re-chosen to parse. **"The test went red" is not "it went red for
  the intended reason."**

## Evidence

Session 2026-08-20T17:11Z–18:10Z, commit `ec2fc2c`.
