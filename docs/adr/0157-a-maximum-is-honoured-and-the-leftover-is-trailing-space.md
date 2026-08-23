---
id: 0157
title: A maximum is honoured, and the leftover is trailing space
status: accepted
date: 2026-08-23
supersedes: []
superseded_by: []
principles: []
tags: [ui]
---

# A maximum is honoured, and the leftover is trailing space

## Context

`karakuri-layout` solves a split by giving each child a size, clamping each to its `[min, max]`,
and repeating until nothing else hits a bound. The interesting question is what happens to the
discrepancy that clamping creates, and it has two directions.

**Short is the easy one.** A split that comes up short would leave a gap, and a gap is not
something its parent has another child to fill, so whatever is still unfrozen absorbs it. That is
what makes children tile their parent exactly, which is the invariant most of this crate's tests
are written against.

**Long is the one that needed deciding.** Where every visible child is already sitting at its
maximum and there is still room, there is nothing left that may legally grow.

It is not hypothetical, and `solo` is where it shows: fold everything but a pane that says it is
never wider than 480, and 480 is what it gets on a 1280-wide window.

## Decision

**A maximum is honoured, and the space nothing may take is left empty after the last child.**

Stated from the other end, so the whole rule is in one place: **a split fills its parent by growing
or shrinking whatever is still unfrozen, and only a maximum stops it.** Trailing space appears only
when every visible child has hit one.

**A consequence worth knowing, because it makes `Fixed` mean less than its name.** `Fixed` keeps its
size *while something else is absorbing the change* — which is the normal case, since a split
usually has a flexible child. In a split where nothing flexible is left unfrozen, the fixed children
scale in proportion to fill their parent, and it is their `max` rather than their size that stops
them. So a pane that must never be wider than 480 says `max(480.0)`; saying `fixed(480.0)` and
nothing else does not say it.

## Alternatives

**Override the maximum for whichever child is left.** Rejected as the worst of the three: a pane
that says it is never wider than 480 is not made 1280 wide by being the only one on screen. A
maximum is a promise the arrangement makes, and there is no operation that undoes a layout deciding
to break it.

**Hand the leftover to one child** — the last, the first, or the one the code happens to reach
last. Rejected for being arbitrary in a way that is invisible: it produces an arrangement nobody
asked for, and which child gets it is an artefact of the loop rather than of anything the operator
or the arrangement said.

**Centre the children in the space.** Rejected for a smaller reason: it makes a child's position
depend on whether a *sibling* is at its maximum, so folding an unrelated pane away moves everything
sideways. Trailing space keeps the origin where the arrangement put it.
