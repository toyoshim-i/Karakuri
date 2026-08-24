---
id: 0160
title: A boundary is a rectangle, not a coordinate
status: accepted
date: 2026-08-24
supersedes: []
superseded_by: []
principles: []
tags: [ui]
---

# A boundary is a rectangle, not a coordinate

## Context

`karakuri-layout` exposed a tree that can be walked downward and solved, and left every question
that goes **upward or across** to whoever called it — the parent of a node, the visible children of
a split, the edge between two of them, all the edges. The first real caller wrote itself a model to
answer them, which is what a crate that owns the arrangement exists to prevent, and four of that
caller's stand-ins became public API of the wrong crate.

This record is about one of those questions, because it had a shape worth choosing rather than
just a gap worth filling.

**`set_divider` said where a drag *landed*; nothing said where a boundary *is*.** A pointer that
picks a divider up needs it, or the divider jumps to the pointer instead of moving with it — the
grab offset is the difference between the two, and it cannot be taken without knowing where the
boundary was. A view also has to *draw* the thing.

## Decision

**`Layout::boundary(split, index) -> Option<Rect>`, and what it returns is the gap**: the space
between the pair, spanning the split across its axis and as thick as its divider.

**A rectangle rather than a coordinate**, for three reasons and the third is the one that decided
it.

It is what a view draws. It needs no second method, because the coordinate `set_divider` speaks in
is `Axis::origin` of the gap — one call, where a scalar accessor beside a rectangle accessor is two
things that will drift.

And **it makes a bug structurally impossible.** The caller's own version checked that visible child
`index` existed and not that `index + 1` did, so folding the far side of a boundary during a drag
returned a position for the split's own far edge — a boundary with nothing beyond it, reported as
though it were one. `Released::Gone` existed for exactly that case and was unreachable through it.
A gap between two children **cannot be constructed without both**, so the check is not something the
implementation remembers to do; it is what the return value means. That is the same species of
argument as [P-0071](../principles/0071-solving-a-layout-never-mutates-it.md), one level down.

## Alternatives

**A scalar `boundary_at(split, index) -> Option<f32>`.** What the caller had. Rejected: a view needs
the geometry anyway, so this is the smaller half of the answer, and adding the rectangle later leaves
two methods where one would do.

**Both.** Rejected on drift: two accessors for one fact, and nothing keeping them agreeing except
that they were written on the same day.

**Leave it to the caller.** What was happening. Rejected by the whole of this pass — the caller
reconstructs the visible-child filter to do it, which is the trap this crate is meant to hold, and
gets the pair check wrong, which is the bug above.
