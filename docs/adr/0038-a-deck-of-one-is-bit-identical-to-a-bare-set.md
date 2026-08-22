---
id: 0038
title: A deck of one is bit-identical to a bare Set
status: accepted
date: 2026-07-30
supersedes: []
superseded_by: []
principles: []
tags: [engine, determinism]
---

# A deck of one is bit-identical to a bare Set

## Context

Introducing the deck put a composite pass between every Set and the output. Bit-exact
reproduction is a standing invariant, and "close enough" would quietly end it.

## Decision

The composite is built so that **a deck of one produces bit-identical output to a bare Set**:

- **Slot index is the compositing order**, fixed. Slots are a `Vec` visited by index, there is no
  `HashMap` anywhere on the path, and the shader's terms are unrolled in slot order. Iteration
  order is not left to a container.
- **`textureLoad`, not `textureSample`** — no sampler is bound at all. A sampler at unit scale is
  *supposed* to be identity and this does not depend on that being true.

Measured, at 262144 elements and 1280×720 on the host clock:

| | median | worst |
| --- | --- | --- |
| Bare Set | 3.98 ms | 13.57 ms |
| Deck of one | 4.24 ms | 10.12 ms |
| Deck of four | 7.98 ms | 10.84 ms |

The composite costs about 0.26 ms; three more slots about 3.7 ms; four slots hold 28.1 MiB of
targets at 8 bytes a texel.

## Consequences

- **A defect surfaced from the deck's own author**: `Deck::begin_frame` also runs
  `HotSwap::begin_frame` for **Allocated** slots, so a candidate that is not on screen is judged
  against the frame interval. It passes a watchdog it never paid, and by the time it is Live and
  paying, it has already been accepted.
- The watchdog itself: **8 warmup frames discarded, then the median of 30**. Warmup covers
  pipeline first use, first buffer touch and the staged upload; **median rather than max**
  because it is a host clock and a single sample carries whatever the operating system was doing
  — the same call the probe makes. Thirty frames is about half a second: long enough not to fire
  on noise, short enough that a bad Set is not on screen for long.
- `--watch` polls mtimes rather than using a filesystem-notification crate, because the worker is
  already a poll loop — it has to wake to free retired Sets — so there is no blocking receive for
  an event to replace.

## Evidence

Session 2026-07-30T19:15Z–19:16Z.
