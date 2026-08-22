---
id: 0099
title: Element 0 is the oldest living element
status: accepted
date: 2026-08-16
supersedes: []
superseded_by: []
principles: [0003]
tags: [ir, engine, render]
---

# Element 0 is the oldest living element

## Context

What an L3 may point at had three candidates, and I had put **index 0** at the top of the table as
*cheap but unstable — compaction moves it*.

## That row was wrong

**Compaction is order-preserving.** Survivors keep their relative order, so if the element at index
0 is alive then zero live elements precede it and **it stays at index 0**. It moves only when it
dies, and its replacement is the next survivor in spawn order — which is age order.

**So index 0 is the oldest living element.** Not an arbitrary slot that happens to be reachable —
something worth pointing a camera at. In a procedure with no `spawn` block it is simply `seed == 0`
unless someone kills it.

Order preservation was chosen for entirely different reasons: a stable blend order and bit-exact
reproduction. **This is its third payment, and none of the three was visible from the other two.**

## Decision

Reductions (centroid, bounds) and **element 0** as the two things an L3 may follow, with **no
declaration and no check** — a convention and its consequences written down, which is what this
language does instead of adding a header field.

What made it land was the other half: **L1 authors work with it in mind.** An L1 that expects to be
watched can give element 0 meaning — spawn it first, never kill it, make it the leader the rest
follow. One that does not care still offers *the oldest survivor*, which is defensible rather than
arbitrary.

## Consequences, written before they were discovered

- **When element 0 dies the camera's subject moves to the next oldest.** For a fountain that reads
  as continuity; for a cloud with a deliberate anchor it is a jump. The fix is upstream — do not
  kill the anchor — and saying so beats machinery that hides it.
- **An empty live range holds the last position** rather than snapping to the origin. Holding needs
  state, which an L3 may have. A hard cut the instant the material runs out is the least explicable
  accident an operator can be handed.

## Evidence

Session 2026-08-16T04:57Z–05:00Z.
