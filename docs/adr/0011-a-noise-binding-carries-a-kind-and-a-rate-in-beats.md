---
id: 0011
title: A noise binding carries a kind and a rate in beats
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: [0090]
tags: [signal, format]
---

# A noise binding carries a kind and a rate in beats

## Context

[ADR-0005](0005-spawn-is-an-accumulator-and-the-engine-owns-the-birth-fraction.md) rejected
Poisson on the grounds that irregularity bound to `spawn_rate` keeps **both its depth and its
period** adjustable. The signal bus was then implemented with per-step white noise: correct,
deterministic, and possessing **no period at all** — fixed at the step rate. The argument that
had defeated Poisson applied unchanged to the implementation that replaced it.

The root cause was in the format: a `bind` record had `curve` and `range` and nowhere to say
how fast.

## Decision

A `bind` carries a **`noise` object**: `kind`, `rate`, `stream`.

- **`kind`** — `white` / `value` / `perlin` / `fbm`, default `perlin`. Names taken from the
  IR's existing builtins rather than invented, because there is no reason for the signal bus
  to spell the same concept differently. Art-side expectation is Perlin first, which is why it
  is the default.
- **`rate`** — **cycles per beat**, not per second. The local oscillator is the single truth
  about phase, and a VJ context cares about beats. Even `white` gains a hold time of `1/rate`
  beats, so no kind is periodless.
- **`stream`** — decorrelates one binding from another. A plain integer field; the earlier
  `noise:<key>` colon syntax was invented by the implementation, existed nowhere in the
  format, and risked colliding with signal-name validation.

Depth stays on the existing `range`.

## Alternatives rejected

- **Keep per-step white noise.** Cannot be tuned, which is what Poisson was rejected for.
- **A bare `rate` field on `bind` for any signal.** More general; adds format surface before a
  second user exists. Revisit if a non-noise signal needs one.
- **Kind alone, without a rate.** Kind gives smoothness; period is a different axis and would
  have stayed an engine constant.

## Consequences

- A degenerate case was closed rather than left: `bpm <= 0` froze every noise silently, since
  the phase is beat-derived. Tempo is clamped.
- **Recorded and deliberately not fixed:** the moment external sync exists, every noise binding
  follows tempo correction, including bindings whose flicker has no musical intent, and there
  is no seconds-relative mode to opt out with. Harmless while V1 has no external input;
  whoever implements external sync chooses between a seconds-relative rate and pinning the rate
  at bind time. Written into the specification rather than left in an agent's report.

## Evidence

Session 2026-07-25T13:15Z–13:31Z, commit `f79bd27`.
