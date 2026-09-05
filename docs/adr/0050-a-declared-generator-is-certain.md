---
id: 0050
title: A declared generator is certain, and a sample is not always in [0,1]
status: accepted
date: 2026-08-01
supersedes: []
superseded_by: []
principles: [0084]
tags: [signal]
---

# A declared generator is certain, and a sample is not always in `[0,1]`

## Context

My brief for signal binding asserted two things, and the implementing agent refuted both.

**"A sample is in `[0,1]`."** False for two signals. `bpm` carries a tempo — 128 — so a curve that
clamps saturates it. Noise is signed, in `[-1,1)`, so clamping discards half of it.

**"Noise binds at confidence 0.1, like every synthesized signal."** The refutation has a source in
the specification. When Poisson was rejected, the argument was that irregularity belongs on a noise
signal bound to `spawn_rate` — offered as **the** alternative, the whole basis of the rejection. If
that binding lands at a tenth of its strength, the alternative is not one, and
[ADR-0005](0005-spawn-is-an-accumulator-and-the-engine-owns-the-birth-fraction.md) collapses.

## Decision

**A declared generator binds at confidence 1.0.** Confidence exists to express a missing provider,
and a generator declared in the record has no provider to be missing — there is nothing to be
uncertain about. Noise maps `*0.5 + 0.5` into `[0,1]` rather than being clipped, and both facts are
written into the specification.

## Consequences

- Confidence is not a general quality score. It answers exactly one question — *is something on the
  other end of this?* — and a value the record itself declares always has an answer.
- `bpm` saturating a curve is a real edge; documenting it was judged insufficient and it is
  diagnosed.
- **A decision one document makes can be load-bearing for another.** The Poisson rejection lived in
  the IR specification and the confidence rule lived in the signal crate, and only reading them
  together showed that setting the confidence low would have quietly repealed the other.

## Evidence

Session 2026-08-01T03:17Z, 2026-08-01T05:31Z.
