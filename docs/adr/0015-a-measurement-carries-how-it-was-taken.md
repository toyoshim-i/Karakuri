---
id: 0015
title: A measurement carries how it was taken
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: [0012]
tags: [engine, process]
---

# A measurement carries how it was taken

## Context

The compaction agent reported that every GPU timestamp read back as zero. Rather than accept
it, the numbers were reproduced directly: the adapter advertises `TIMESTAMP_QUERY` and
`TIMESTAMP_QUERY_INSIDE_ENCODERS` as true, a real bug was found and fixed (the device was not
requesting the second feature) — **and it was not the cause**. A load that cannot be zero, a
million invocations by two thousand trigonometric iterations, measured through both the encoder
path and the pass-descriptor path, returned literal zeros in every query.

The eventual finding was worse than "does not work": the probe agent showed it is
**unstable** — the same heavy load returns a plausible value, or zero, or a negative, run to
run.

This mattered because a standing invariant says performance-touching changes come with GPU
timestamp measurement, and stage 8 promotes a pipeline on that number. **0.0 ms reads as a
very fast shader.**

## Decision

A `Measurement` **carries how it was obtained**, the same way a signal carries confidence. A
degraded timing source is a fact the consumer can see, not a number that silently means
nothing.

Detection is of the **actual degeneracy, not the feature flag**: calibration requires ten
consecutive readings above a threshold before the timestamp path is trusted. On this machine it
consistently falls back to the host clock, and says so.

## Alternatives rejected

- **Trust the feature flag.** It is true here and the readings are garbage.
- **Fall back silently.** Produces the failure this decision exists to prevent: an unmeasurable
  shader promoted for being fast.
- **Fail hard with no timestamps.** Leaves the whole project unbuildable on the only machine it
  is being built on.

## Consequences

- The environment fact — Apple Silicon plus wgpu advertises timestamps and does not deliver
  usable ticks — is written into the module documentation rather than carried in someone's head.
- Two agents reached the conclusion independently, which is corroboration rather than one
  agent's excuse for a zero.

## Evidence

Session 2026-07-25T15:00Z, 2026-07-25T15:42Z. Standing rule:
[P-0012](../principles/0012-a-measurement-carries-how-it-was-taken.md).
