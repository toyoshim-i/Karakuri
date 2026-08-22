---
id: 0016
title: Agents leave work in the tree; the reviewer commits
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: [0017]
tags: [process]
---

# Agents leave work in the tree; the reviewer commits

## Context

V1 was implemented by a team of agents. The question was how their output enters history.

## Decision

Agents are told **not to commit**. Work is left in the tree; the reviewer runs the tests,
reads the code, fixes what is wrong, and commits. History therefore contains only reviewed
work.

Parallelism is bounded by a second rule: **the seam is cut serially first.** The shared
types three passes agree on — the typed IR, the builtin table, the binding layout between
codegen and engine — are defined before anything fans out, because agents left to invent
them produce incompatible ASTs and incompatible bind group layouts, and the merge costs
more than the parallelism earned. The binding layout in particular must be *one* type, so
it is published as a public contract that the engine consumes rather than a text the engine
greps.

## Alternatives rejected

- **Let agents commit their own work.** Cheaper, and the defects below would be in history.
- **Split by crate and run everything in parallel.** Collides with the standing preference
  for vertical slices over layers. The resolution used both: a vertical slice driven serially
  while the pure crates were filled in parallel against a fixed seam.

## Consequences

Defects caught before entering history, in one day:

- A **false rejection of valid code**. The brief said to detect signal-bus names at parse
  time; `energy` is not a reserved word, so `param energy : float [0,1] = 0.5` is legal and
  was being rejected. The agent followed the instruction *and reported that it was wrong*,
  which is the behaviour being selected for.
- The `Cost` seam mistake in
  [ADR-0013](0013-cost-has-three-axes-that-must-not-be-added.md), reported by the agent
  that had to build on it.
- A **requirement walked back with an argument.** An agent was told to keep a test proving
  that one step of `2·dt` and two steps of `dt` differ. Making noise tempo-relative removed
  that property; the agent said so rather than dropping the test, and replaced it with a
  test asserting the opposite. On review the agent was right: substepping exists so that
  state at a given time does not depend on frame rate, and a signal that varies with tick
  granularity means a slow machine looks different rather than dropping frames. Determinism
  holds either way, so no test would have caught it.

## Evidence

Session 2026-07-25T12:56Z–15:00Z. Standing rule:
[P-0017](../principles/0017-only-reviewed-work-enters-history.md).
