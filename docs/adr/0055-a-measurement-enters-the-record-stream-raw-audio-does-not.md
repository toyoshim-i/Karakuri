---
id: 0055
title: A measurement enters the record stream; raw audio does not
status: accepted
date: 2026-08-01
supersedes: []
superseded_by: []
principles: [0002, 0084]
tags: [signal, determinism, format]
---

# A measurement enters the record stream; raw audio does not

## Context

Live input cannot be reproduced, and the standing invariant is that the same record stream and the
same seed reproduce the same output bit for bit. Audio appeared to be in direct conflict with it.

## Decision

**Put the measurement in the record stream.** `tick` is the exact precedent: derived from real time
when live and written down, read back verbatim on replay with nothing measured
([ADR-0006](0006-the-step-count-is-a-record-not-a-measurement.md)). Audio takes the same shape,
which needs new record types the v0.2 vocabulary does not have.

**Recording the raw audio and re-running the analyser at replay is explicitly refused.** The analyser
is expected to improve, and **a session recorded today must replay identically after it does.**
Recording the analyser's *output* makes the analyser replaceable; recording its *input* freezes it.

**A quiet room and a missing microphone are different things.** An `energy` of 0.0 at high confidence
is a measurement — silence — and a frame with no audio record is a frame with no provider, not a
frame of silence. Whether a binding stays sane when someone unplugs the interface mid-set is decided
by that distinction.

## Consequences

- Refined a week later, and the refinement matters: what is recorded is **the correction rather than
  the estimate**, which is what lets the analyser improve without changing how an old session
  replays.
- This closes the gap noted when `--bpm` was added with no record to map onto
  ([ADR-0046](0046-a-flag-writes-into-the-record-it-does-not-invent-one.md)).

## Evidence

Session 2026-08-01T06:01Z, 2026-08-01T06:50Z. Standing rules:
[P-0002](../principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md),
[P-0084](../principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md).
