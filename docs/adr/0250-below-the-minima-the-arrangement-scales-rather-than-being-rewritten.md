---
id: 0250
title: Below the minima the arrangement scales rather than being rewritten
status: accepted
date: 2026-09-04
supersedes: []
superseded_by: []
principles: [0082]
tags: [ui, layout]
---

# Below the minima the arrangement scales rather than being rewritten

## Context

P-0071 — *Solving a layout never mutates it* — is retired into
[P-0082](../principles/0082-looking-never-writes-back.md), *Looking never writes back*, whose body
is the general rule: a read produces an answer and leaves what it read alone.

Two things P-0071 carried are narrower than that rule and have to keep an address.
[ADR-0156](0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md), the record P-0071
named, states the rule in one clause — *a viewport too small for the minima produces small
rectangles and changes nothing stored* — and settles who owns the arrangement. It does not say what
the small rectangles are, and it does not name the alternative that loses. This record does both, so
that retiring the file loses neither.

**A minimum is a claim about a viewport that can hold it.** The console's row of three declares
minima summing to 700 plus 8 of divider, and its column 272 plus 8. A window can be narrower than
that — dragged there, restored there from a session saved on a larger screen, or arrived at when a
projector disconnects and the desktop reflows — and the solve still has to return a rectangle for
every region. Something has to give, and the choice is *what*.

## Decision

**The minima give, in proportion, and nothing stored is touched.**

A child below its declared minimum is raised to it and frozen, where the declared minimum is first
capped by what the node's visible content can use
([ADR-0174](0174-a-node-claims-only-what-its-visible-content-can-use.md)). When the frozen sizes
then come to more than the split's available extent, the last step of `solve_split` scales **every**
child by `avail / total` — the frozen ones included, which is what makes the shortfall shared rather
than dumped on whichever child was still flexible. The dividers shrink with them: where the dividers
alone would exceed the extent, each is `extent / gaps`.

**Nothing goes negative anywhere on that path.** `avail` is clamped at zero, the scale factor is
zero when there is nothing to scale, and each rectangle is sliced from `size.max(0.0)`, so a `0 x 0`
viewport produces zero-sized rectangles rather than inverted ones and every downstream reader — hit
testing, clipping, the boundary a drag reads — keeps working on them.

**And the arrangement is untouched throughout.** `Layout::solve` destructures itself into the
arrangement and the solved buffers and passes the arrangement by shared reference, so a line that
stored a solved size into a node does not compile. The stored 240 and 320 are still 240 and 320 at a
`1 x 1` viewport, and the window coming back reproduces the rectangles **exactly** rather than
nearly — `tests/arrangement.rs::shrinking_below_the_minima_and_growing_back_reproduces_the_arrangement`
sweeps eight viewports down to `0 x 0` and compares every rectangle in the arena for equality, and
`a_drag_survives_the_viewport_collapsing_and_coming_back` does the same with an operator's drag in
place of the defaults.

## Alternatives rejected

**Clamp the stored size during the solve.** *The pane cannot be 240 here, so it is 180 now.* One
line, and at the moment it runs it is indistinguishable from the right thing — the rectangles it
produces are the same rectangles. It is a slow leak: every excursion into a small viewport
permanently rewrites what the operator arranged, and there is nothing to restore it from, because
the only copy was overwritten by a resize nobody asked to be an edit. **It fails only across time**,
which is why it survives review — nothing at the moment of the change looks wrong, and the loss
shows up as an arrangement that has drifted over a week with no event to blame.

**Store the rectangle a drag produced.** The same write from the other direction, and it is what a
divider drag makes shorter. The borrow in `solve` refuses it; `tests/arrangement.rs` covers the
routes the types leave open, which are the operations.

**Enforce the minima and let the last child go negative or overflow.** What a layout that treats a
minimum as a hard constraint does when the constraints cannot all hold. It moves the failure into
every consumer of a rectangle instead of into the picture, where it is visible and recoverable by
making the window bigger.

## Consequences

- **A declared minimum is a preference below the viewport that can hold it, not a guarantee.** A
  caller may not read `min` as *this region is never smaller than this*; what it can read is that
  nothing else got the space.
- **P-0071 retires with no loss.** Its general half is P-0082; its layout half is here; who owns the
  arrangement is ADR-0156, unchanged.
- **Nothing in the code changed for this record.** It describes what `karakuri-layout` already does
  and what its tests already assert, written down because the file that held it is being deleted.
