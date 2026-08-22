---
id: 0041
title: A candidate is judged only on the frames it contributed to
status: accepted
date: 2026-07-31
supersedes: []
superseded_by: []
principles: []
tags: [engine]
---

# A candidate is judged only on the frames it contributed to

## Context

`Deck::begin_frame` ran every slot's `HotSwap::begin_frame`, and every frame interval was fed to
the watchdog — including for slots that were not on screen. Measured: a candidate in an
**Allocated** slot completed a full 8-warmup-plus-30-frame trial and was **rolled back at
1.388 ms**. A Set that took zero steps and rendered zero pixels, judged on its neighbour's cost.

I had flagged the false-**accept** direction: a candidate passing a trial it never paid for. The
review pointed out that the false-**reject** direction is the same bug and is **worse** — a tight
budget with busy neighbours **silently discards good generated material**, and in a system whose
whole point is generating material, that is the expensive failure.

## Decision

An off-air slot still installs finished builds and hands retired Sets to the worker, but **takes
no sample and resets its frame reference**. The trial **freezes** and resumes when the slot goes
Live.

Stated as semantics rather than as a patch: **a candidate is judged on the frames it actually
contributed to.**

## Consequences

- What this still cannot do, documented in place rather than left to be discovered: **one Live
  slot's cost cannot be separated from its neighbours'.** Every Live slot is judged against the
  whole deck's frame interval, so a tight `--budget-ms` rolls back every candidate in a deck of
  four. Fixing that needs per-Set measurement, which is the governor's.
- A related retirement, found by a later review: a **hot-swapped build did not retire the slot's
  meter**, so the outgoing Set's light was reported as the incoming Set's with `frames_behind: 1`
  asserting freshness. The texture does not change, so nothing in the generation machinery
  noticed. `Swapped` and `RolledBack` now retire; `Accepted`, `Rejected` and `WorkerLost`
  deliberately do not.

## Evidence

Session 2026-07-31T12:04Z, 2026-07-31T15:23Z.
