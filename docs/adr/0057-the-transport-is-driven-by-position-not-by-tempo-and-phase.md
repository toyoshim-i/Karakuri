---
id: 0057
title: The transport is driven by position, not by tempo and phase
status: accepted
date: 2026-08-01
supersedes: []
superseded_by: []
principles: []
tags: [engine, signal]
---

# The transport is driven by position, not by tempo and phase

## Context

The requirement was not physics. It was **tape** — beat-synced fast-forward and rewind as an effect,
with "the picture runs on in real time while an effect layer is beat-locked" as one option among
several rather than the shape of the system.

That reframing corrects a premise I had built on. Fast-forward and rewind are not *time running at
another speed*; they are **evaluating the procedure at a different `t`**, and rewind is time moving
**backwards**.

## Decision

**Drive the transport with a position**, not with a tempo and a phase. A position can go backwards
and it can jump; a tempo and a phase cannot express either.

While only acoustic analysis exists, that position is supplied as *forward integration of the
tempo* — a degenerate, always-forward case. When a deck is connected, a real position enters the
same door. **The same shape as signal binding: the provider is replaceable without rebuilding
anything downstream.**

**Scratch information does not come from the microphone**, and this is the distinction that decides
the design. Acoustic analysis yields a tempo and a beat phase, and it is a **forward-only estimate**.
During a scratch the analyser sees something that no longer looks like a beat, and the correct
response is its **confidence collapsing** — it does not emit a reversed grid. Only the deck itself
can say that time moved backwards.

| Provider | Carries | Can go backwards | When |
| --- | --- | --- | --- |
| Acoustic analysis | tempo + beat phase | no | now |
| Deck link | the playhead position | **yes** | M7 |

## Consequences

- Per slot, the transport is a map from session time to that slot's `t`. Default is identity.
  "The picture runs in real time and the effect layer is beat-locked" is then **the same mechanism at
  two settings** — the lower layer's map is identity, the upper one's is synced. No special case.
- Scratching is where the confidence rule pays off exactly as designed: with analysis only,
  confidence drops during a scratch and the oscillator free-runs; with a deck attached confidence
  stays high and the position moves back and forth. **Neither path asks whether a provider exists.**
- What acoustic analysis specifically cannot do for the deck link is left uncommitted: what PRO DJ
  LINK actually sends during a jog is not asserted without a real device in hand.
- Authoring guidance falls out of it, concrete enough to put in a generation corpus: **material you
  want to scratch must be written closed-form.**

## Evidence

Session 2026-08-01T07:00Z–07:20Z.
