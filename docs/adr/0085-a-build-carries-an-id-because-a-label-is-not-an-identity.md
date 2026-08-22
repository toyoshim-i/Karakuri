---
id: 0085
title: A build carries an id, because a label is not an identity
status: accepted
date: 2026-08-12
supersedes: []
superseded_by: []
principles: []
tags: [engine, format]
---

# A build carries an id, because a label is not an identity

## Context

[ADR-0084](0084-a-procedure-change-goes-into-the-record-stream.md) looked like one record. It was
not, because **the CLI could not say which build had landed.**

`Event::Swapped { label }` carries a display string — `"drift_shell + soft_points"` — which is
**identical for every rebuild of the same pair**. And the queue **collapses superseded builds**: a
Set that is overtaken is never shown, so the number submitted and the number landed do not match and
counting cannot pair them either.

Reading the source from disk at the moment of landing was considered and rejected: the file may have
changed again in between, and it would record a lie. **A feature that exists for fidelity cannot use
"almost right".**

## Decision

`swap::Request` carries an **`id: u64`**, echoed on every `Event` about that build. The CLI keeps
`id → (l1 hash, l4 hash)` and records against the id that actually landed.

**This is not a trick for the recorder.** An asynchronous build queue should have had it, and
`Request`'s own documentation was already reaching for it:

> a request that depends on what happens to be live is not reproducible from a record stream

## And the record lands at `Swapped`, not at `Accepted`

`Accepted` is the verdict thirty frames later. Recording there would put the replay **thirty frames
late in the ordinary case** — the one where nothing was rolled back at all. Recording at `Swapped`
makes the ordinary case exact and shifts only the case that was rolled back.

## A named infidelity

**A rollback cannot restore `t`.** Live, the previous Set returns at the `t` it was parked at; a
replay rebuilds it from the hash, so `t` returns to zero. `Set::seek` only works for closed-form
material, so this is not solvable in general.

It is **recorded and accepted** rather than papered over, for a reason that bounds it: a swap *in*
is defined as starting cold, so it reproduces exactly — only a rollback differs, and a rollback
means the budget was exceeded, which is an exceptional frame to begin with.

## Evidence

Session 2026-08-12T17:12Z, commit `85f9b24`.
