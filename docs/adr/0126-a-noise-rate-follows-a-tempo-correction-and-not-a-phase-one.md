---
id: 0126
title: A noise rate follows a tempo correction and not a phase one
status: accepted
date: 2026-08-02
supersedes: []
superseded_by: []
principles: [0029]
tags: [signal, determinism]
---

# A noise rate follows a tempo correction and not a phase one

## Context

[ADR-0011](0011-a-noise-binding-carries-a-kind-and-a-rate-in-beats.md) fixed a noise rate at **cycles
per beat**, and recorded the consequence it could not yet settle: *the moment external sync exists,
every noise binding follows tempo correction — including bindings whose flicker has no musical
intent — and there is no seconds-relative mode to opt out with.* Harmless while nothing corrected the
oscillator.

Audio input and beat tracking landed, so the oscillator is now corrected against tracked audio and
the consequence is live.

## Decision

**Made rather than deferred: a noise rate stays cycles per beat and follows a *tempo* correction; it
does not follow a *phase* correction.**

The mechanism was already there. The oscillator carries **two beat counts** — musical position, which
takes phase shifts, and elapsed beats, which does not — and **noise reads the second.**

- **A tempo correction is a rate change**, so it reaches noise continuously with no jump at the
  instant it lands. That is what the accumulators are for.
- **A phase correction is the beat grid being realigned with a room.** Re-hashing every noise stream
  because of one would make a flicker with no musical intent **jump every time the tracker nudged the
  grid** — and it nudges several times a second.

## Alternatives rejected

Both were the ones ADR-0011 had written down as the likely answers, and **both fail for the same
reason**: a seconds-relative mode, and a rate frozen at bind time.

**They make two kinds of noise, and every existing `bind` record becomes ambiguous about which kind
it asked for.** A record that could mean either thing is worse than a record that means one thing
imperfectly — the same objection that keeps one record tag to one record shape
([P-0031](../principles/0031-a-name-means-one-thing-across-the-system.md)).

## What is given up, stated rather than glossed

The claim that **a noise lattice point coincides with a beat instant after a correction**. Nothing
depends on it, and nothing can observe it.

## Evidence

Session 2026-08-02T08:59Z; normative text at `docs/ir-spec.md`, "Binding noise". Recovered later than
the rest of that day — the pass over 2026-08-02 missed it, and **removing the duplicate summary from
the specification is what surfaced the gap.**
