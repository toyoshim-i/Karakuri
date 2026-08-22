---
id: 0013
title: Cost has three axes, and they must not be added
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: []
tags: [ir, engine]
---

# Cost has three axes, and they must not be added

## Context

A seam mistake, reported by the agent that had to build on it. `Cost` exposed a single
`ops_per_element`, so the cost pass summed every block in a procedure. The agent flagged the
consequence — a procedure with a cheap `element` and an expensive `spawn` is penalised every
frame — and called it conservative.

It was not conservative. It was wrong. `spawn` runs **once in an element's life**, `element`
runs **every frame**, and the frame's spawn work is proportional to `spawn_rate * dt` rather
than to the population. The two quantities do not scale with the same thing, so they are not
addable numbers. L4 has the same defect one step further: `fragment` is per covered pixel, and
the specification already said in prose that a per-element figure for L4 is fiction.

## Decision

Three axes, never summed:

| Axis | Charged per |
| --- | --- |
| `ops_per_element` | live element × frame |
| `ops_per_spawn` | once in an element's life |
| `ops_per_fragment` | covered pixel |

Three ceilings, calibrated separately: spawn is loose because it amortises, fragment is tight
because one sprite covers tens of pixels.

**The type's documentation says they must not be added and what happens if they are**, because
they share a unit and adding them will otherwise look reasonable.

## Alternatives rejected

- **One conservative total.** Penalises exactly the procedures the design encourages —
  expensive initialisation, cheap steady state.

## Consequences

- `perf` metadata takes two forms: L1 records `ns_per_element` and `bytes_per_element`; L4
  records a measured `ms` with the resolution, capacity and parameters it was measured at.
- The weights behind the estimate are **ordinal inventions, not measurements**, and are
  documented as such — individual numbers could be off by 2–3×. This is acceptable because the
  probe at stage 7 is the authority; the estimate exists to reject the obviously impossible
  cheaply.

## Evidence

Session 2026-07-25T13:45Z–14:24Z.
