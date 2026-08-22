---
id: 0026
title: The deck is L5's surface, and its size is two budgets
status: accepted
date: 2026-07-26
supersedes: []
superseded_by: []
principles: []
tags: [engine, ui]
---

# The deck is L5's surface, and its size is two budgets

## Context

An audit found the same idea under five names in the roadmap — priming Sets in M2, a slot's
variant pool in M4, a staging lane in M5, an agent's pool in M6, and wherever look-ahead
generation lands in M7 — with nothing saying whether they are one mechanism or five, and
**nothing anywhere saying how many may be resident at once**.

## Decision

**The deck is not a new data model. It is the surface that shows and operates L5's state**, and
it is the same store at different granularities. Loaded from the library, some of it active and
mixed to output — a Resolume grid rather than a DJ player.

It maps exactly onto the lifecycle that already existed:

| Where | Lifecycle state |
| --- | --- |
| Library | Cold — on disk, not even compiled |
| Deck | Warming / Priming — compiled, allocated, running unseen |
| Active (1–4) | Live — composited to output |
| Just pulled down | Cooling |

**The deck is the only place the lifecycle becomes visible.** Priming is by definition unseen —
a reduced-rate render taking a stateful simulation to its attractor. Without the deck there is no
surface that answers "is this ready to bring up", and finding out by bringing it up and watching
particles being born is not something you do in front of an audience. The roadmap already argued
that an unobservable autonomous system is unusable on stage and therefore the interface precedes
the agents; the **same argument fires one milestone earlier**, because priming exists before any
agent does.

**Its size is two separate budgets**, not one:

- **VRAM** — hard. At 262144 elements a Set is roughly 50 MB, so eight slots is about 400 MB.
  This decides how many slots exist.
- **Compute** — dynamic. How many of those slots may actually step. This is the governor's.

So the slot count is a UI constant and **residency level is the governor's output**. Three levels:
**Live**, **Priming** (running hidden at reduced rate), **Allocated** (compiled and allocated,
**not stepping**, memory only).

## Consequences

- The governor is promoted from useful to **required**: it must be able to say "eight resident is
  not possible at this capacity", which it cannot do while it only controls priming *quality*.
- Position and attention on the deck are governor input — the slot you are about to bring up
  primes at full rate, the one held back for later trickles.
- A **variant pool is a family of pre-forked Sets sharing most of their slots**, so switching one
  is a Set switch and is cheap because everything else is identical. Its price depends on which
  slot varies: three variants differing only in L4 share one simulation; three differing in L1
  are three simulations.
- M5's staging lane, M6's agent proposals and M7's look-ahead scheduling all write into the deck
  rather than each inventing a waiting area. **Where material waits is one place, and who put it
  there is not asked.**

## Evidence

Session 2026-07-26T02:41Z–07:24Z; audits at 2026-07-26T02:53Z and 02:55Z.
