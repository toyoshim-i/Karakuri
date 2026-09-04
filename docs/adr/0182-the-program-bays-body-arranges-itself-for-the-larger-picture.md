---
id: 0182
title: The Program bay's body arranges itself for the larger picture
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: [0239]
tags: [ui]
---

# The Program bay's body arranges itself for the larger picture

## Context

[ADR-0181](0181-the-picture-is-the-canvass-shape-and-the-leftover-is-the-consoles.md) gave the
picture the canvas's shape, so a wide Program bay now has **ground down each side**. The request:
put A and B down the left and C and D down the right, with the caveat that more constraints are
needed before a resize gives a unique answer.

## Decision

**Whichever placement gives the larger picture wins**, ties to *below* — the arrangement the mock
draws. It is stateless and nothing is stored, so
[P-0082](../principles/0082-looking-never-writes-back.md) is untouched, and it is one
function of a rectangle: `program_body(body, canvas)`.

Verified against the numbers rather than argued:

| body | below | beside | winner |
|---|---|---|---|
| 466 x 333 — the mock's narrowest | 466 x 262 | picture box **−131.3**, so not available | Below |
| 1396 x 333 — a 1920 window | 466 x 262 | **592 x 333** | Beside, and the picture is **1.61x** |
| 800 x 333 | 466 x 262 | 202 x 114 | Below — no silly flip near the crossover |

**The crossover is a curve in (width, height), not a width**, and that was worth finding: it sits at
body width 1064 — a 1588-wide window — but at 1396 wide the arrangement flips *back* to below at
body height 427, and **a bay dragged to its own minimum goes beside at every width, the mock's
narrowest included**. One flip along each axis and never a flicker: beside's picture is
single-peaked in the height and below's is monotone, and both sweeps assert that rather than
asserting a threshold.

### A side column is a function of the height alone

Two cells stacked down the body with one gap between them, the column being one of them at
`PREVIEW_ASPECT`: `(H − 6)/2 × 16/9`. Both properties were checked by sweep rather than by
argument. It is **not so greedy that beside never wins** — the column does not follow the width, so
every extra pixel of width goes to the picture. It is **not so mean that a cell is unreadable** —
a cell beside is 163 tall against the row's 63, and 73 even at the bay's minimum height, so the
arrangement that moves the cells makes them *larger*.

**It is greedy in the other direction and that is the cost**: a tall bay widens both columns until
below wins again. Beside is the answer for a bay that is **wide and short**. A cap was rejected —
it would reintroduce a dependence on the width, which is the defect the rule exists to avoid, and
would give two crossovers along one drag.

**Two columns of two, not one of four**, and the arithmetic points the other way: one column of four
is far cheaper (140 wide against 581). It is chosen for the picture staying centred with equal
ground either side, and for a cell being half the height rather than a quarter — 163 against 79.

### A cell's place is its deck's, not its turn's

Asked whether, with fewer than four previews running, the left column fills first or the two
alternate, the answer is **neither, and ADR-0170 had already settled it**: a cell is drawn whether
or not a deck is behind it, and *"an empty cell is what off looks like, not a stand-in for a full
one"*. So `C · off` sits at the top of the right column whether or not C runs, and turning B off
does not slide C up. **The letter is the only thing naming a deck, and a label that moves when a
neighbour stops is a label nobody can point at.** The function is handed no liveness at all, which
is that rule written as a signature.

A, B down the left and C, D down the right is the row's own left-to-right order folded in half, so
C moves from third-in-the-row to top-of-the-other-side rather than somewhere new.

### Two gaps, neither invented

Picture to column is `.program-body`'s `gap: 8px`; between two stacked cells is `.previews`'s
`gap: 6px`. CSS's `gap` shorthand sets the row and column gaps alike, so each declaration states its
number for the across direction too. There is no third: the columns sit on the body's own edges,
which are already `.program-body`'s padding.

One number was unavoidable — the preview row's height when it is below — and it is **derived**
rather than transcribed, so [ADR-0179](0179-a-transcribed-number-cites-the-rule-it-was-copied-from.md)'s
guard checks the four declarations it is derived from.

## Consequences

- **The crate had written its tiling rule twice and knew it.** The mixer's `track` carried a doc
  saying it was *"the same reading `preview_cells` takes"*, beside a hand-written copy of that
  arithmetic in `preview_cells`. One function now, three call sites. The `None` rule was three
  copies of one sentence, each citing `picture_rect`; two are one function now.
- **`mod gpu` holds a sentence this makes false**, at the exact two widths it already uses: *"a cell
  stays exactly as big at any wider window"* — its two widths straddle the crossover.
- **This is only half.** Nothing draws it yet, because drawing it needs something from
  `karakuri-layout` that does not exist, and that is a decision of its own rather than a
  consequence of this one.
