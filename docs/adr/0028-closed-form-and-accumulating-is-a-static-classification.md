---
id: 0028
title: Closed-form and accumulating is a static classification
status: accepted
date: 2026-07-26
supersedes: []
superseded_by: []
principles: [0022]
tags: [ir, engine]
---

# Closed-form and accumulating is a static classification

## Context

Two ways of working were observed: material whose picture at time `t` is a pure function of `t`,
and material that has to be integrated forward. The suggestion was to expose both a zero-origin
elapsed time and a per-frame delta so a procedure could use whichever it needed.

## Decision

**The observation is right and needs no new ambient.** Elapsed time is `t`; the per-frame delta is
`dt`; a pause is `{"steps": 0}`, which was already expressible.

What was missing was **the classification, and using it**. A procedure is **closed-form** if it
never reads an attribute it emits, and **accumulating** otherwise. The check pass already tracks
attribute reads and writes, so the answer costs nothing new, and the compiler records it in the
artifact's metadata.

It decides lifecycle:

- **Closed-form** — **no priming at all.** Any `t` is reachable directly, so Cold to Live is
  immediate, and cueing and scrubbing are free.
- **Accumulating** — must be run to its attractor, so it has to be warmed on the deck.

So the Allocated-versus-Priming argument only applies to accumulating material, and a closed-form
Set can be brought up without occupying a deck slot at all. It also decides where beat-resolution
variant switching is actually usable: instant selection needs every candidate resident, which for
accumulating candidates means paying for each — but a closed-form candidate costs nearly nothing
to keep, so **beat-resolution switching is a closed-form feature**.

## Alternatives rejected

- **Make `dt` the measured elapsed time.** Determinism rests entirely on `dt` being fixed, and
  replay, undo, A/B comparison and session recording go with it — four properties, one change.
- **Add ambients for elapsed and delta time.** Both already readable.

## Consequences

- Pulling a Set from Live to Allocated keeps its state and stops it stepping; returning to Priming
  **resumes rather than restarts**. Nearly free to implement — do not release the buffers, stop
  advancing. And because `t` is simulation time, five minutes parked advances it by nothing, so it
  resumes at the same instant. [ADR-0006](0006-the-step-count-is-a-record-not-a-measurement.md)
  pays off here.
- Left as homework: a param bound to a signal keeps moving while its Set is parked, so the value
  jumps on resume. Smoothing may be needed.

## Evidence

Session 2026-07-26T07:35Z–07:39Z. A real bug was found in the same pass — `steps` was not driving
the compute pass, so a pause did not pause and time drifted from state under load. The determinism
tests passed because both sides of the comparison were **equally wrong**, which is the limit of a
test that compares an engine against itself. Fixed in `555da34` with two regression tests. Standing
rule: [P-0022](../principles/0022-closed-form-material-needs-no-warming.md).
