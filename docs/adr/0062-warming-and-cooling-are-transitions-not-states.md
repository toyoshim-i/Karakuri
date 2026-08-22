---
id: 0062
title: Warming and Cooling are transitions, not states
status: accepted
date: 2026-08-02
supersedes: []
superseded_by: []
principles: []
tags: [engine]
---

# Warming and Cooling are transitions, not states

## Context

The lifecycle was specified with five states — Cold, Warming, Priming, Live, Cooling. Three were
built, and checking the roadmap against the code asked why.

## Decision

**Three, because the other two were never states.**

- **Warming** has nothing to be. A Set arrives at the deck **already compiled** — that is what the
  build worker is for — so there is no interval during which it is becoming ready. It is Cold, and
  then it is a Set.
- **Cooling** is likewise not a condition a slot is in. A slot taken down is simply **no longer
  stepped**. Nothing decays and nothing needs to finish.

## Consequences

- A model invented before the implementation names the phases the author imagined. Some of them turn
  out to be **edges rather than nodes**, and finding out is part of building it.
- Recorded in the roadmap as three built out of five named, with the reason — rather than as three of
  five done, which reads as work outstanding.

## Evidence

Session 2026-08-02T08:59Z, commit `5ad1d1b`.
