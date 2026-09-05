---
id: 0110
title: This machine is not the reference
status: accepted
date: 2026-08-19
supersedes: []
superseded_by: []
principles: [0088]
tags: [process, engine]
---

# This machine is not the reference

## Context

I had argued that narrowing the element slot could wait, on the evidence that a four-slot deck fits
comfortably here. **This machine is comparatively rich, and VRAM is expensive right now.**

**It is the second time I have used this machine as evidence** — the first is recorded two sections
away, where it advertises a GPU timestamp feature it does not actually have.

## Decision

Measured, and the old paragraph was **factually wrong twice** as well:

- **"VRAM doubles" is overstated.** Under `std430` a `vec3` sits on a 16-byte boundary whatever the
  layout does, so only scalars are recoverable — 80 B to 64 B for `drift_shell`, **a fifth, not a
  half.**
- **The element layout is not what decides whether a deck fits at the default capacity.** One slot is
  57.8 MiB, a four-slot deck 231 MiB — and one `amplify 6` stage adds **120 MiB**, `amplify 64` adds
  **1.25 GiB**.

**Amplification multiplies exactly the stride the layout shrinks.** Taking a fifth off is taking a
fifth off the largest allocation in the system — which is the reason to do it, and it was hidden by
*it fits here*.

**The rule, with a test for telling the two apart:**

> No hard limit is set, and waste that is theoretical at design time is still removed. **Does using
> it buy anything?** Capacity buys elements. A resident Set buys instant switching. Sixteen bytes
> holding a four-byte `seed` **buys nothing at any size on any machine** — so it is not a budget
> question and does not wait for a budget.

**4 GiB of dedicated VRAM is the figure a design is checked against, not a ceiling.** Beyond that
the performer's machine decides. And **development happens on a desktop while a performance is
likelier a laptop**, which is the same correction one level up.

## Evidence

Session 2026-08-19T13:32Z. Standing rule: none in
`docs/principles/`. P-0088 stated it and was split on 2026-09-05 — reading this machine as evidence
about this machine is a discipline for whoever is working rather than a property of the engine, so
it is stated in `docs/contributing.md` §1, *Working style*.
