---
id: 0148
title: A variant pool is a Set, and the deck stays a mixer
status: accepted
date: 2026-08-23
supersedes: []
superseded_by: []
principles: []
tags: [design, deck, engine]
---

# A variant pool is a Set, and the deck stays a mixer

## Context

M4 described a variant pool as "pre-forked Sets sharing every other slot, so they are deck
members rather than a separate structure". Two of its four cost claims are only true inside
one Set and two only across deck slots, and cross-Set geometry sharing — which would have
reconciled them — is rejected
([ADR-0147](0147-geometry-is-not-shared-across-sets.md)).

## Decision

**A pool is a Set.** The alternatives are nodes of one Set and selection picks among them;
the deck is not where a pool lives. The half built in `094a264` is the design rather than the
cheap corner of it.

**The deck stays a mixer.** Selecting at the deck is a cut. Fading between two things is what
an operator does at the final stage with the deck's own faders and masks, which already exist
and already schedule on the grid — a second fade mechanism inside the selection would be the
same gesture spelled twice.

**The deck holds four, and the number stays movable.** Four is the current design and not a
question worth much: what matters is that nothing hard-codes it in a way that makes changing
it a rewrite. `MAX_SLOTS` and the compositing shader's four inputs are two caps that happen
to agree today — membership and *Live* — and they should stay separately statable so that a
later major version can move one without the other.

## Alternatives

**A pool spanning deck slots**, as the roadmap described. Rejected: with geometry unshared it
buys nothing an operator cannot get by putting two Sets in two slots and crossfading, which
the deck already does; and it would spend the instrument's four slots on alternatives of one
thing.

**Selection as a scheduled fade.** Rejected above — the deck already fades, and a pool that
faded too would put the same gesture in two places with two spellings.

## Consequences

**A composited Set has to round-trip.** `Layering` is deliberately not in the Set file, on the
rule that a Set file records the nodes of a Set and not how they meet each other — which is
why the shipped selection says outright that it cannot be saved. If a pool is a Set, that
caveat is the next thing to remove, and removing it means either bending that rule or finding
a reading of it that admits `Layering`. **That is the next decision this line of work owes,
and it is not made here.**

**An L1 alternative is still expensive.** `Set::step` walks every source, so alternatives that
differ at L1 are stepped whether selected or not. Separate simulations are the point for those
rather than an inefficiency (P-0092), but nothing yet lets one be parked while another runs —
so a pool of L1 alternatives inside one Set is not the cheap thing an L4 pool is, and the
roadmap should stop implying it would be.
