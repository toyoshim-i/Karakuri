---
id: 0053
title: Priming runs the simulation and skips rendering entirely
status: accepted
date: 2026-08-01
supersedes: []
superseded_by: []
principles: []
tags: [engine]
---

# Priming runs the simulation and skips rendering entirely

## Context

The roadmap specified priming as *"rendering hidden, at reduced rate and reduced resolution."*

## Decision

**That is wrong, and the reduced resolution does not exist.**

What needs warming is **per-element state, and all of it belongs to L1.** L4 is stateless — it reads
whatever L1 last wrote. So priming runs `prepare` and the L1 pass and **skips rendering entirely**.
There is no target, and therefore no resolution to reduce.

**"Reduced rate" survives, with a different meaning.** Under budget pressure a priming slot advances
on some frames only. Its `t` then falls behind real time, and **that is correct**: it is catching up,
not keeping a clock.

## Consequences

- Cheaper than specified, by the whole render cost of every priming slot.
- It is also **forward jump**. Running a slot without showing it is exactly what a jump forward
  needs; shown it is a fast-forward effect, hidden it is catching up. One mechanism, two ways of
  exposing it — see [ADR-0058](0058-closed-form-is-worth-more-for-scrubbing-than-for-priming.md).
- A roadmap written before the implementation describes a mechanism that does not exist yet, and
  will sometimes describe it wrongly. It is the plan, not the specification, and correcting it is
  part of building the thing.

## Evidence

Session 2026-08-01T06:01Z.
