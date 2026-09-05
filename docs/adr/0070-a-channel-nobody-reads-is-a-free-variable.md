---
id: 0070
title: A channel nobody reads is a free variable
status: accepted
date: 2026-08-08
supersedes: []
superseded_by: []
principles: [0087]
tags: [render, engine]
---

# A channel nobody reads is a free variable

## Context

Before `over`, L4's alpha blend was `Zero * src + One * dst` — **nothing was ever written to the
alpha channel**, and `present` and the meter both read only `.rgb`. Entirely dead data.

Giving it a reader made every value the material can put there an input. The IR says alpha is
straight and **values above 1.0 are expected**, and no pass clamps. `over` computes
`A·(1 − covered)`, so a coverage of 1.5 does not hide — **it subtracts**; at 2 and above the sign
inverts and it amplifies. Measured: **1029 negative colour channels, worst −54.34.**

## Decision

Saturate coverage in the composite: `covered = opacity * clamp(src.a, 0, 1)`, computed once and used
for both the hiding term and the alpha composition. Clamping in L4 instead would change the colour
too, since the additive blend multiplies the same alpha into it.

The general form is the part worth keeping: **a value nobody reads is unconstrained, and it has been
out of range the whole time with no way to notice.** The defect is created by the reader, not by the
writer.

## Consequences

- **A control tested only at its endpoints is untested.** `opacity` had never been exercised at
  anything but 0 and 1: **deleting opacity from the composite entirely left the whole suite green.**
  Three tests were added, one per mode, each with an independent reference and each verified to fail
  when its own arm drops opacity.
- The same gap was still open for `gain` via records, found on a second review pass — **my own fix's
  argument had not been carried to the neighbour it also applied to.**
- Recorded as an argument rather than a measurement: containment of a NaN alpha relies on `clamp`
  suppressing NaN, which is **Metal's behaviour and not a WGSL guarantee** — WGSL defines `clamp` by
  `min`/`max`, which propagate. On a backend that propagates, `over` returns to NaN and the suite
  stays green. Written into the code as such.

## Evidence

Session 2026-08-08T16:42Z–17:51Z, commit `55016f2`. Standing rule:
[P-0087](../principles/0087-name-the-property-never-the-shape.md).
