---
id: 0090
title: A ceiling calibrated for one shape rejects the next
status: accepted
date: 2026-08-15
supersedes: []
superseded_by: []
principles: [0091]
tags: [ir, render]
---

# A ceiling calibrated for one shape rejects the next

## Context

The fullscreen renderer was written, and **the cost model forbade ray marching outright.**

The fragment ceiling was 512 operations. That number is a **stand-in for not knowing how many
fragments there will be** — capacity times sprite area times overdraw, an unbounded product. A
fullscreen renderer is **exactly one canvas, exactly once**. There is no unknown left to stand in
for, and a marcher is thirty-two to sixty-four iterations by its nature, so the proxy did not
constrain the feature — **it prohibited it.**

## Decision

Fullscreen uses the same ceiling as an element (4096). Two fragment ceilings, each calibrated for
what it actually bounds.

Generalised and written into the roadmap **before the next case arrives**:

> A ceiling calibrated for one shape rejects the next.

`blend weighted` is the next thing to meet that table.

## Consequences

- **The same cost model then produced an optimisation rather than a rejection.** The example's first
  draft rotated the sample point inside the loop and came to 7022 ops; the hint named `rot_y` as 18%
  of the estimate, and rewriting it to rotate the ray and the eye once brought it inside. The first
  time the estimator's output was a way forward rather than a refusal — the same property
  [ADR-0086](0086-a-hint-says-why.md) is about.
- `examples/field_march.kir` uses **six of the eight SDF builtins that had passed the checker since
  M1 with nothing able to draw them.**

## Evidence

Session 2026-08-15T19:13Z. Standing rule:
[P-0091](../principles/0091-cost-is-known-before-it-is-paid.md).
