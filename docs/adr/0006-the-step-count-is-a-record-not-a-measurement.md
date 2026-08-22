---
id: 0006
title: The step count is a record, not a measurement
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: [0002]
tags: [engine, determinism, format]
---

# The step count is a record, not a measurement

## Context

Substepping — running the simulation more than once per frame so that state at a given time
does not depend on frame rate — appeared to conflict with determinism, because the step count
would be derived from real-time lateness.

## Decision

Put the step count **in the record stream**: `{"t":"tick","steps":1}`.

Live, the engine derives it from real time and **emits** it. Replaying, it reads the number
back and derives nothing. The existing invariant — the record stream is the only path that
changes engine state — absorbs substepping without modification.

v0.2 **writes the record from the start, always with `steps: 1`**. Adding real substepping
later is then purely an engine change: no format change, no determinism change. This is the
same move as declaring `blend additive` before a second blend mode exists
([ADR-0008](0008-blend-is-declared-now-so-the-second-mode-is-an-addition.md)).

The cap is **4**. Past it the simulation is allowed to fall behind, because unbounded
catch-up turns a load spike into a death spiral. `t` advances by `steps * dt`.

## Alternatives rejected

- **Derive the step count at replay from the replaying machine's timing.** Makes a replay a
  re-run, and the faster machine sees a different performance.
- **Defer substepping entirely to a later version.** Costs a format change later; writing the
  record now costs one line.

## Consequences

- `t` is **not wall clock**, and once the cap engages it diverges from it permanently. Stated
  in the specification, because otherwise it is reported as a bug.
- Params and signals are sampled **once per frame** and held constant across substeps. Written
  down at decision time specifically so that "shouldn't these interpolate per substep?" does
  not reopen during implementation.
- Forced the Set file and the session stream apart — see
  [ADR-0007](0007-a-set-file-is-a-projection-and-a-session-stream-is-the-timeline.md).

## Evidence

Session 2026-07-25T12:03Z–12:24Z. Standing rule:
[P-0002](../principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md).
