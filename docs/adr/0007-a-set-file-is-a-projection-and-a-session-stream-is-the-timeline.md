---
id: 0007
title: A Set file is a projection; a session stream is the timeline
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: [0013]
tags: [format, store]
---

# A Set file is a projection; a session stream is the timeline

## Context

[ADR-0006](0006-the-step-count-is-a-record-not-a-measurement.md) put a `tick` record in the
stream every frame. The only ndjson file the specification defined was `.set.ndjson`, so
taken literally the **definition** of a Set would become a timeline hundreds of thousands of
lines long. The `README` already used the phrase "record stream" as though it were a
different thing; it was, and nothing had said so.

## Decision

Two files, defined apart:

- **A Set file** is a **state projection with no time in it**. It carries what a Set is.
- **A session stream** is the **timeline**: the Set, plus `tick`, plus every edit at its frame
  position.

**Saving a Set is a projection of a session**, folding state last-write-wins per layer and key
and dropping the ticks. Not a second authoring path — a derivation.

## Alternatives rejected

- **One file for both.** Makes a Set unopenable and conflates a definition with a performance.
- **Keep ticks out of any file and re-derive them.** Defeats ADR-0006.

## Consequences

- A parameter edit acquires an exact frame position, so replaying a live performance is
  faithful rather than approximate — a benefit that was not the motivation.
- The store enforces it: writing a Set rejects any record that is not state, and it rejects it
  in the projection path too.
- **Keeping them apart is what stops saving a Set from saving a performance.**

## Evidence

Session 2026-07-25T12:06Z–12:24Z; enforcement in `karakuri-store` the same day
(`StoreError::TickInSet`). Standing rule:
[P-0013](../principles/0013-a-set-is-a-projection-and-a-session-is-the-timeline.md).
