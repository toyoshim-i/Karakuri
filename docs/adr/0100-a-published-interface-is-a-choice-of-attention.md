---
id: 0100
title: A published interface is a choice of attention, not of authority
status: accepted
date: 2026-08-16
supersedes: []
superseded_by: []
principles: [0052]
tags: [ir, format, ui]
---

# A published interface is a choice of attention, not of authority

## Context

A Set chooses which controls to expose on the desk, fixing some internally and tying others to
functions. Writing it down produced three consequences.

## Decision

**Publishing decides what is shown. It does not decide what can be reached.** `param` records keep
reaching controls that were never published — which is how a Set file records the author's frozen
values, how `--param` works, and how MCP or an agent adjusts something not on the console.

If publishing were access control, **a Set's author could lock the operator out of their own
machine.** This project's position has been the opposite everywhere else: show every measurement and
act on none, hide nothing, needing a hand is craft rather than defect.

> A surface is a choice of attention, not a choice of authority.

**A published range narrows; it does not redefine.** A `.kir` declaring `[0.0, 8.0]` published as
`[0.2, 0.8]` says *this is all the stage needs*. Outside the declared range is **refused rather than
clamped**, because the declared range is the procedure's claim about where it still looks like
itself.

**An empty interface publishes everything**, which is today's behaviour — so the feature is purely
additive and every existing Set file keeps working. The first entry written is what turns the list
into an interface.

## Consequences

- **Macros arrive with no new record and no new semantics.** `bind` is already *source → curve →
  range → param*; letting its source be *a signal **or a published control*** is the whole feature.
  Bus signals mix by confidence; an operator's hand is confidence 1.
- **`SetError::ParamCollision` disappears rather than being fixed.** Two renderers over one geometry
  almost always both declare `exposure`, so the check **forbids the very purpose of this milestone**.
  An interface gives two namespaces — internal (node plus name, which the graph needs anyway) and
  external (the name the Set gives) — so two `exposure`s can be two published controls or one
  driving both. **Which is right is the author's judgement, not an error.**
- The desk still shows what a Set did not publish: `gain`, `opacity`, `blend` and `mask` belong to
  the **edge into L5** rather than to the Set, and residency, transport and preview belong to *a Set
  being performed*. A Set that publishes nothing can still be mixed.
- The example written into the specification is the point of the whole thing: a Set that merges two
  pipelines through a nested L5 can publish **that L5's crossfade as one control**. Two scenes, one
  knob on the desk, and the twenty numbers that made them stay where the author put them.

## Evidence

Session 2026-08-16T05:00Z–05:03Z. Standing rule:
[P-0052](../principles/0052-publishing-is-a-choice-of-attention-not-of-authority.md).
