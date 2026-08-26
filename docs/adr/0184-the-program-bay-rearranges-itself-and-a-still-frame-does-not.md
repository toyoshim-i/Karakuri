---
id: 0184
title: The Program bay rearranges itself, and a still frame does not
status: accepted
date: 2026-08-26
supersedes: []
superseded_by: []
principles: []
tags: [ui]
---

# The Program bay rearranges itself, and a still frame does not

## Context

[ADR-0182](0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md) worked out where
the picture and the four cells go — whichever placement gives the larger picture, ties to below.
[ADR-0183](0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md) gave
`karakuri-layout` a second bit so a node can be out of the layout for a reason that is not the
operator's. This is the two of them wired together and drawn.

## Decision

**`rearrange` is solve, decide, write, solve**, and it returns whether the bit moved. It is called
from the only two places that are *before anything reads a rectangle*: `plan_into`, so drawing can
never skip it, and the window loop's first act, before the sinks are aimed.

**A still frame does no work.** Both solves are the opening flag test; the write carries the value
the node already has and marks nothing dirty; what is left is a dozen divisions and two fits, no
allocation, and `Repaint::Never`.

**The frame it changes costs two solves, and that is stated rather than hidden.** Solve, derive,
write (dirty), solve. It is unavoidable without deriving the placement from last frame's rectangle,
and a placement changes only when the bay's rectangle does — a resize, a drag, a fold — which is
what P-0072 does not budget.

**Nothing chases its own tail**, and the reason is structural: the bit is derived from the **bay's**
rectangle, and the bay is `Fixed(378)` over a flexible child, so what it can use is unbounded
whether or not the row is set aside. One write is a fixed point rather than the first step of a
chase, and *the second ask finds nothing to do* is asserted at five widths.

### The guard is the shape of the answer rather than a check

*Rearrange only while the picture is visible* — because a split's `usable` is zero when none of its
children is laid out ([P-0073](../principles/0073-a-node-claims-only-what-its-visible-content-can-use.md)),
so setting the row aside while the picture is folded would make the whole bay claim nothing, against
the manual's *"the deck previews under it are auditions of their own, so they stay when it goes"*.

It is not written as a test. **The bay's three states are its own two folds**: both laid out gives
ADR-0182's arrangement, the row folded gives the picture the body whole, and **the picture folded
puts the row below at its own height off its own region** — ADR-0174 unchanged. Removing the guard,
injected, makes the bay flip 72 → 0 → 72 → 0, one flip per frame.

### `Change::Rearranged { moved }` — `Now` when it moved, `Never` when it did not

[ADR-0165](0165-the-repaint-decision-is-one-closed-list.md) makes `Change` one closed list so
omission cannot compile, so this earns an arm rather than borrowing one. It is **not `Viewport`**:
the bay rearranges off *its own* rectangle, which a divider drag, a fold that gives it height, a
solo, or a canvas of another shape all move with the window unchanged — and **naming a change after
its usual cause is how the unusual cause reaches no repaint**.

`Now` when it moved, because every rectangle in the bay is a new one — the picture's and all four
cells' — which is `Viewport`'s argument one bay down. `Never` when it did not, **and that is the
arm's whole point**: this is the only `Change` a caller raises *every frame* rather than on a
gesture, so `Now` regardless would ask for a frame on every frame and cost the still panel the
clause the module exists for.

## Consequences

- **One derivation, four readers.** `picture_rect`, `preview_rects`, `rearrange` and the drawing are
  field accesses on one answer about the bay. `preview_rects` had to move: it inset the
  `deck-previews` *region*, which past the crossover has no extent — so it answered `None`, deck A's
  audition stopped being drawn, and the loop stopped asking for frames.
- **The crossover costs one texture.** Deck A's cell goes 112x63 → 290x163 — **6.7x the texels** —
  remade once, with the old registration freed. A wider window inside the *beside* arrangement
  remakes nothing, because a column follows the bay's height and not its width.
- **`Kind::Previews` outlived its stated reason.** It exists because *"`View::draw` has to know which
  pane the cells go in"*, and past the crossover they are not in that pane. The kind survives for a
  different reason and its arm draws nothing; the cells are painted after the plan loop, because the
  row is not in the plan at all.
- **`Kind::Picture` said *"what is beside it is the bay's card, and nothing is drawn there"***. Four
  cells are drawn there now.
- **`PREVIEW_ASPECT`'s doc spent its own reason.** It said making a cell canvas-aware *"means
  putting a canvas on `View` … that is its own pass and this is not it"*. This is that pass. The
  number stands on ADR-0182 now.
- **`picture_rect`'s `None` rule argued the opposite of what it needed.** Its doc records that the
  visibility test was *deleted* because the size test carried it — *"learnt from this test rather
  than assumed"*. Off the bay that is false: the bay keeps its 378 whatever the picture does. The
  test is back, reading the operator's bit and deliberately not `visible`.
- **ADR-0183's *"nothing in `karakuri-console` had to move"* was wrong**, and only became visible
  here. The console's test checker is a second copy of the layout crate's, with the same defect that
  record fixed — it asked *the operator folded it* for *does this take extent and a divider*. It
  bites the moment anything is actually set aside, which is now.
- **ADR-0183's *"until whoever set it aside says otherwise"* had no *whoever*.** It does now:
  soloing the row while it is beside the picture leaves it holding the viewport with zero height,
  and the frame's own first act lays it out again. Asserted, with the intermediate state asserted
  too.
- **`placed.len() == REGIONS.len()` was two claims in one number** — *the table is complete* and
  *every region is drawn*. They part company the moment a region is set aside, and they are two
  assertions at two windows now.
- **Two test sweeps and two sentences straddled the crossover without knowing.** Three of the five
  widths in `tests/view.rs`'s sweeps are past it while both tests asserted region-relative facts at
  all five; the picture's own gpu test widened by the same 400 from 1440 that crosses it; and
  *"widen the window and the picture does not follow"* was written twice, true *within* an
  arrangement and false across the step. `docs/manual/console.html` carried the same sentence and is
  corrected, with a paragraph describing the rearrangement — the manual's other claim,
  *"they stay when it goes"*, is still exactly true, and it is true **because** of the guard.
