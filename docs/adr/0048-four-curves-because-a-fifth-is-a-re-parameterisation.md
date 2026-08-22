---
id: 0048
title: Four curves, because a fifth is a re-parameterisation
status: accepted
date: 2026-07-31
supersedes: []
superseded_by: []
principles: []
tags: [signal, ir]
---

# Four curves, because a fifth is a re-parameterisation

## Context

A `bind` shapes a signal through a curve. The specification fixed `lin` and `pow2`. How many more
to add was open, and the pull is always toward more.

## Decision

**Four, and the argument is that they are four distinct shapes** of a monotone `[0,1] → [0,1]` map,
not four names:

| | Shape | What it is for |
| --- | --- | --- |
| `lin` | flat | the identity |
| `pow2` | peak-weighted | flattens the floor, so a `beat` reads as a hit |
| `sqrt` | floor-weighted | `pow2`'s exact complement — lifts a signal that lives near the bottom, which **every confidence-scaled invented signal does** |
| `smooth` | S | zero derivative at both ends, for a param that is a position rather than an intensity |

A fifth — `pow3`, `expo` — is a **re-parameterisation of one of these**: a name with no behaviour
behind it.

**And a name without a behaviour is paid for twice in a specification an LLM generates against.**
Once when the model has to choose between two spellings of the same shape, and again when the
resulting corpus is inconsistent for no reason.

`sqrt` earns its place on a system-specific ground rather than a general one: because a bound signal
is scaled by confidence, an invented signal *lives near the floor*, and without a floor-weighted
curve there is no way to make one legible.

## Evidence

Session 2026-07-31T23:48Z.
