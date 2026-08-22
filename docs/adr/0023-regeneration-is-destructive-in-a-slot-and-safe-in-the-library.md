---
id: 0023
title: Regeneration is destructive in a slot and non-destructive in the library
status: accepted
date: 2026-07-26
supersedes: []
superseded_by: []
principles: []
tags: [process, store]
---

# Regeneration is destructive in a slot and non-destructive in the library

## Context

Iterating on a creative work with a model has a known failure: the rewrite silently changes the
parts you liked. The concern was raised and then corrected — the units being generated are small
building blocks, and a destructive result is acceptable by design.

## Decision

**Regeneration may be destructive, because the design already bounds what it can destroy.**

- **Blast radius is one slot.** Regenerating L3 leaves L1, L2 and L4 structurally untouched, and
  the Set still points at the same hashes. This is what the typed slot interface buys — the same
  reason `L2 : Geometry -> Geometry` composes.
- **The old version does not disappear.** Content addressing plus `parent` means a slot-level
  destructive change is library-level non-destructive. Undoing a bad regeneration is **one
  record**: point the `slot` back at the previous hash.
- **A bad procedure cannot run away.** Four validation stages stop it; if it passes, the probe
  and the promotion gate stop it; if it goes live and exceeds budget, the governor rolls it back.
  Three independent blocks, which is *why* generation is allowed to be destructive.

## Consequences

- **Staging can be A/B rather than a merge UI.** Show the new one against what is live and let
  someone choose; the old one still exists. This makes the staging lane substantially cheaper
  than designed.
- The first supplier to the staging lane is **a person regenerating a slot**, not an agent. It is
  therefore useful — and testable — before any autonomous system exists, which is a milestone
  earlier than it was placed.
- Two gaps remain, both small: slots can typecheck and still not suit each other aesthetically
  (a `point_scale` tuned for a dense shell is wrong for a sparse lattice) — that is quality, not
  safety. And the **instruction that produced version N+1 was not being recorded**; only the
  final prompt was, which loses the path. `origin` gains it.

## Evidence

Session 2026-07-26T02:26Z–02:27Z.
