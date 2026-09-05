---
id: 0219
title: The crossfader spans the selection and the one after it
status: superseded
date: 2026-08-29
supersedes: []
superseded_by: [0233]
principles: [0090]
tags: [ui]
---

# The crossfader spans the selection and the one after it

## Context

`Operation::Crossfade { from, to }` names both decks, and its own documentation says *"the next deck
is the keyboard's translation of this, not the operation"* — the `x` key resolves it as
`from = focus`, `to = (from + 1) % slot_count`. **Nothing said which two decks the panel's
crossfader spans**, and the mock hardcodes `A … B`, which says nothing at all about a mixer with
four.

The gap was found by drawing it: the crossfader turned out to be already drawn — it is `.xfade`'s
first row, and `karakuri-console` already counts it as 61 of the mixer's 316 — so what it lacked was
a name, a tooltip, and an answer to this question.

## Decision

**The panel's crossfader spans the deck that is selected and the one after it, which is exactly what
`x` means on the keyboard, so the two are one gesture rather than two that disagree.**

[P-0090](../principles/0090-a-surface-offers-it-never-decides.md) is what makes this
the page's to decide: *a surface owns the affordance, never the authority.* Which two decks a control
spans is an affordance over an operation that names both explicitly; the vocabulary is untouched.
And [the operations page](../manual/operations.html) states the standing that lets it be written
there — *"where a badge names a region, that is where the control lives in the console … naming one
is a change to this specification rather than a note on it."*

**A note on what "the selection" is.** It exists in the specification — `Operation::SelectDeck`, the
`0`–`3` keys, the solid ring the mock draws — and **not in `karakuri-console`'s code**, whose `view`
says outright *"nothing here selects a deck"*. That is the manual being ahead of the implementation,
which is the arrangement this milestone is built on rather than a hole.

## Alternatives rejected

**Fixed `A … B` ends**, which the mock already drew. It says nothing about the third and fourth
decks, and it decouples the panel from `x`: the same gesture would mean one thing under a finger and
another under a key, which is the manual's first rule broken at the affordance rather than at the
operation.

**Assignable ends** — the convention on every hardware DJ mixer, where the operator says which two
channels the crossfader spans. It is a real design and it loses on cost against need: it is a new
piece of console state, and [P-0090](../principles/0090-a-surface-offers-it-never-decides.md)
wants state to end in a record, so it would want a record nothing else asks for — to serve a need
that two `SelectDeck` presses already meet.

## Consequences

- **One sentence is still owed by the page** and is named here rather than left to be met: which
  direction a click on the *near* end means. The operation can say `Crossfade { from: next, to:
  selection }`, the keyboard cannot say it at all, and the tooltip's *"Click an end to ask for a
  crossfade"* leaves it open.
- **The crossfader's mark stays a readout**, decided separately and on
  [P-0090](../principles/0090-a-surface-offers-it-never-decides.md)'s terms: it is one number
  derived from the two channel faders' recorded values, and a draggable mark would have to invert a
  projection that is not invertible — inventing the split between two `SetOpacity` records by a law
  no record names. That answers the tension
  [ADR-0180](0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md) recorded and left
  open: *"one of those needs a different name or a different control."* It got a different control.
- **Annotated 2026-08-31: the control this record is about was removed.**
  [ADR-0233](0233-the-consoles-mixer-has-no-crossfader.md) decided that the console's mixer has no
  crossfader, so nothing spans two decks and the sentence owed above is owed by nobody. This is
  annotation and not supersession — the subject went, the decision was not reversed, and no record
  answers *which two decks* differently
  ([P-0066](../principles/0066-an-adr-is-a-description-of-history-corrected-but-never-revised.md)).
