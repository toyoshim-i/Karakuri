---
id: 0047
title: A binding blends on confidence
status: accepted
date: 2026-07-31
supersedes: []
superseded_by: []
principles: [0029]
tags: [signal, engine]
---

# A binding blends on confidence

## Context

`bind` records existed in the format and moved nothing. The standing invariant is that a signal is
always complete and **a consumer branches only on confidence, never on whether a provider exists**.
That had never been given a mechanism.

## Decision

Every frame, a binding:

1. **Samples the signal by name.** This cannot fail and does not return an `Option` — an unknown
   name yields a synthesized value.
2. Applies the curve over `[0,1]`.
3. Maps into the declared `range`.
4. **Blends on confidence:** the value written is `lerp(the param's own value, the mapped value,
   confidence)`.
5. Writes the uniform. No fork, no recompile.

**Step 4 is the whole of "consumers branch only on confidence."**

**The manual value is the base of the blend and is never overwritten.** `params` stays the manual
store and the binding's result is computed into a side field, read at write time. So `--param` on a
bound parameter is not a competing writer: at confidence 1.0 it carries no weight, at 0.1 it keeps
90%, at 0.0 it is exactly the answer. **Order-independent by construction, not last-writer-wins.**
At most one binding per (layer, param); a second replaces the first, because two would be resolved
in vector order.

**The oscillator lives on the `Deck`** — one per session — and `Frame::render` advances it once per
frame, by the same clamped `steps` every Live slot advances by, before any slot is prepared.
`Set::prepare` takes `&Signals`, so **a Set cannot resolve a binding without being handed the
session's clock** and no per-Set oscillator can be created by accident. An `Allocated` slot is not
prepared, so it reads no signals while parked and rejoins the deck's beat rather than a phase of its
own.

## The consequence that was deliberately not softened

Binding to `energy` barely moves anything, because with no microphone its confidence is low. **The
demo looks dull.** The brief said so explicitly:

> Ignoring confidence would make this lively, and it is a lie you would have to write out again
> later.

`beat` and `bar` work completely, because the local oscillator is the invariant's single truth
about phase. An `energy` that behaves convincingly with no microphone attached is far worse than an
unexciting demo.

## Evidence

Session 2026-07-31T16:00Z, 2026-07-31T23:48Z. Standing rule:
[P-0029](../principles/0029-a-consumer-branches-only-on-confidence.md).
