---
id: 0098
title: L5 is one node kind with two roles
status: accepted
date: 2026-08-16
supersedes: []
superseded_by: []
principles: []
tags: [engine, ui]
---

# L5 is one node kind with two roles

## Context

The operator's description settled it in a sentence: L5's proper use is **the desk an operator
mixes on**, and using it nested inside a Set to combine several L4s is **extra expressive range**.

## Decision

**One node kind, two roles, one implementation.** The only difference is whether a control surface
is attached to it.

L5 is **built in rather than written**: L1, L2 and L4 make material and L5 arranges it — *something
you operate, not something you author* — which is the distinction the deck already embodies.

**And that split something that had been one thing.** Two sets of properties had been living
together in `Deck`:

- **Mix properties** — `gain`, `opacity`, `blend`, `mask`. These belong to **an edge into an L5**,
  and they travel with it when it is nested.
- **Deck properties** — residency, priming, hot swap, the budget governor, transport, preview,
  meters. These belong to **a Set being performed**, and have nothing to do with mixing. They sit
  next to L5 only because that is where performance happens.

With the distinction in hand, what a nested L5 should carry stops being a question: the first list
only.

Structurally the deck's own mix *is* the same operation at the top level, and a deck slot is an edge
into a top-level L5 — but they are deliberately **not identified**, because the deck carries the
performance surface. The difference is not structural; it is what is exposed on the desk.

## Evidence

Session 2026-08-16T04:52Z–04:57Z.
