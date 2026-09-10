---
id: 0049
title: `Slot` means two things, and the clash is recorded rather than resolved
status: superseded
date: 2026-07-31
supersedes: []
superseded_by: [0344]
principles: []
tags: [docs]
---

# `Slot` means two things, and the clash is recorded rather than resolved

## Context

`slot` names two different positions: a **layer's position inside a Set**, and a **Set's position
in the deck**. The roadmap calls the second a *member*; the code calls it `Deck::slot`.

## Decision

**Neither is renamed. The discrepancy is written into the vocabulary.**

Renaming either one today is pure labour with no reader helped, and choosing which to rename
requires knowing which meaning the eventual GUI and record format lean on — which is not known.
Deciding later, when something actually needs the distinction, is cheaper and better informed.

What is not acceptable is the collision being *undocumented*, so a reader meeting both meanings
concludes one of the two texts is wrong.

## Alternatives rejected

- **Rename now for consistency.** Costs a sweep, risks choosing the losing name, and buys nothing
  until something depends on telling them apart.
- **Say nothing.** The cheapest and the one that produces the confusion.

## Consequences

- A general shape worth naming: when a decision is not yet forced, **recording the ambiguity is a
  decision**, and it is different from either resolving it or ignoring it. It leaves the choice to
  whoever has the information, without letting the next reader waste an hour deciding which
  document is stale.

## Evidence

Session 2026-07-31T15:41Z.
