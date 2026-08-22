---
id: 0027
title: A Set value is immutable; its compiled instance is not
status: accepted
date: 2026-07-26
supersedes: []
superseded_by: []
principles: [0004]
tags: [engine, determinism]
---

# A Set value is immutable; its compiled instance is not

## Context

Immutability was challenged, and reasonably: editing is done in the background while previewing,
and reallocating buffers and recompiling on every keystroke would be unusable. The invariant's
own wording was ambiguous — *"Every structural change goes through forking a Set. Never mutate a
**live** Set in place"* — unconditional in the first clause, qualified in the second.

## Decision

**Split the value from the instance.**

- **A Set value is immutable**, content-addressed, and appears in records.
- **A compiled instance** is the thing on the GPU, and a change needing neither reallocation nor
  recompilation **may be applied in place**.

Parameter values — the blend amount between two sources included — are not structural changes and
never fork at all.

Editing in the background then costs nothing, because "mutate the background Set" and "fork on
each edit and discard the intermediates" are indistinguishable from outside: the store is
content-addressed, so an unsaved intermediate simply never exists.

## Alternatives rejected

- **Make a non-live Set genuinely mutable.** It opens a hole in the record stream — an unrecorded
  change would alter engine state — and replay stops reproducing. Replay, undo, A/B comparison and
  session recording all rest on that one property, and all four are lost together.

## Evidence

Session 2026-07-26T07:22Z–07:24Z. Sharpens
[P-0004](../principles/0004-a-live-set-is-never-mutated-in-place.md).
