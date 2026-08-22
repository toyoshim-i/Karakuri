---
id: 0054
title: The governor budgets from the probe, and never touches a Live slot
status: accepted
date: 2026-08-01
supersedes: []
superseded_by: []
principles: [0033]
tags: [engine]
---

# The governor budgets from the probe, and never touches a Live slot

## Context

The GPU-timestamp question had been deferred since
[ADR-0015](0015-a-measurement-carries-how-it-was-taken.md). The governor forced it.

## Decision

**The governor budgets from the cost the probe measured at build time** — not from live timestamps,
and not from the deck's frame interval. All three grounds were already in the repository:

- Timestamps are not trustworthy on this machine, and `probe.rs` explains at length how it works
  around that.
- **The deck's frame interval cannot separate one slot's cost from its neighbours'** — which is
  precisely the defect a review had already found in the swap watchdog
  ([ADR-0041](0041-a-candidate-is-judged-only-on-frames-it-contributed-to.md)). Using the same
  quantity for the governor would have reproduced it deliberately.
- `swap.rs`'s worker already submits and waits before handing a Set over, so there is a place to
  measure that is not the render thread.

**And the governor never touches a Live slot.** Loading four heavy Sets is the operator's decision;
a governor pulling a slot off air during a set is **the worst behaviour it could have**. It warns.

## Alternatives rejected

- **Steer on the host clock.** Reacts to submission overhead as though it were the Set's cost.
- **Build no automatic demotion at all**, and let the operator choose the resident count. Named at
  the time as the honest option for M2, and beaten only because the probe already exists and cost is
  already designed to be an intrinsic property of an artifact.

## Consequences

- One `Probe` is built lazily and reused. Calibration is expensive, and **two probes can disagree
  about whether this adapter's timestamps work** — which would make their numbers incomparable, the
  exact property a governor summing them must not lack.
- The measurement rides the same channel as the Set and is parked and restored alongside it.

## Evidence

Session 2026-08-01T06:01Z, 2026-08-01T07:22Z. Standing rule:
[P-0033](../principles/0033-the-governor-never-takes-a-live-slot-off-air.md).
