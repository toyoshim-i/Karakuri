---
id: 0209
title: The mask's shape keeps its empty MIDI badge, and what would reopen it is a control that shows an angle
status: accepted
date: 2026-08-28
supersedes: []
superseded_by: []
principles: [0031, 0090]
tags: [midi, vocabulary, surfaces, mixing, docs]
---

# The mask's shape keeps its empty MIDI badge, and what would reopen it is a control that shows an angle

## Context

[ADR-0202](0202-the-map-reaches-the-masks-front-and-the-shape-has-no-spelling.md) landed
`cc N -> mask-position D` and left the other half of the mask deliberately undecided. It named three
ways out and costed all three without taking one — a float form in the map grammar, an angle-less
`SetMaskShape { deck, kind }`, or the row keeping its gap with the reason on the page — and said why
it stopped there: *"the evidence favours the third for now, and weakly … Taking the decision before
that writer exists is choosing between two grammars on no evidence."*

The writer it was waiting on has since arrived.
[ADR-0203](0203-the-mask-chip-carries-the-angle-it-does-not-control.md) made the console's mask mini
a control, and it hands back the angle the slot is already wearing rather than inventing one — which
is the one answer a pad has no way to copy. So the question is no longer *which grammar*; it is
whether anything has asked for one, and nothing has.

**This record takes the third option.** It is a decision rather than a continuation of ADR-0202's
survey, which is why it is a record of its own: ADR-0202 correctly describes a moment when three
options were open, and P-0066 says an argument that would have to change is a new record rather than
an edit. Its front matter is annotated the way ADR-0185, ADR-0188, ADR-0197 and ADR-0198 were, and
its prose is untouched.

## Decision

***Set a deck's mask shape* keeps `MIDI —`, and the page says the badge is settled rather than
outstanding.**

Nothing in the grammar, the vocabulary or `karakuri-midi` changes. What changes is that the page
stops describing a mechanism and states a decision, in the two places it already keeps reasons of
this kind — the row's own prose and *What the gaps say* — and both now say what would reopen it: a
control that **shows** an angle.

## Alternatives rejected

- **A float form in the map grammar**, so a line could read `note 62 -> mask-shape 0 linear 0.785`.
  It is the only option that leaves `SetMaskShape` exactly as ADR-0201 wrote it, and it is a small
  parser change. It loses on the format's own discipline: a value in a map line is a word out of a
  list [the vocabulary owns](../principles/0090-a-surface-offers-it-never-decides.md),
  and **a number is the one kind of value that can be checked against nothing** — so the refusal an
  operator gets for a typo stops being *write one of these* and becomes silence, or a wipe at an
  angle nobody meant. It also raises a question no surface can answer today: the angle is in
  radians, and nothing anywhere shows one, so what a person is meant to type is undefined.
- **An angle-less `SetMaskShape { deck, kind }`.** It makes the shape a pure value-word target the
  day it lands, which is the tidiest grammar of the three. It loses because it changes a row that
  landed two days ago and moves the angle somewhere worse: to a reading, on a crate that reads
  nothing back by construction, or to a default — which is precisely the fault ADR-0201 was written
  to avoid and P-0078 makes worse rather than better, since `Record::Mask` is a state and a replay
  cannot tell a squared-off front from a hand on one.
- **Keep the question open and decide when the console's mask mini is finished.** This is what
  ADR-0202 did, and repeating it now would be a survey mistaken for a decision. The mini is finished
  (ADR-0203) and it did not ask for a map target; leaving the row's reason written as a mechanism
  invites the next reader to re-derive the same three options from the same evidence.
- **Fold this into ADR-0202 as an edit.** It is where a reader would look, and it is what P-0066
  forbids: ADR-0202's argument is that the evidence does not separate three options, and that
  argument would have to change. The annotation is what connects them.
- **A new principle.** The rules this applies already exist — a surface asks for what it can say
  (ADR-0192), an operation names a value out of a list the vocabulary owns (P-0090) — and this is a
  decision about one row rather than a rule anybody could violate.

## Consequences

- **No counts move.** The page is still 49 operations and 212 ways in, of which 51 exist; MIDI still
  reaches 8. Recounted from the page itself after the edit.
- **The page's two mask rows now read as a pair rather than as a contradiction**: the position has
  `cc → mask-position N` and the shape's empty badge carries the reason and the condition that
  would change it, so a reader does not take one as an oversight beside the other.
- **What stays open is named as a trigger rather than as work**: a control that shows an angle. If
  one is drawn, the choice between a float form and an angle-less operation is live again and this
  record is what it argues against.
- **ADR-0202 is annotated and not revised** (P-0066), and `karakuri-midi`'s module documentation
  still states the same fact about the grammar, which this does not change.
