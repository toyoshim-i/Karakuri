---
id: 0181
title: The picture is the canvas's shape, and the leftover is the console's
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: []
principles: []
tags: [ui]
---

# The picture is the canvas's shape, and the leftover is the console's

## Context

`picture_rect` returned the whole `program-view` region and `Present::draw` letterboxed the canvas
into it. So on any window wider than the mock's narrowest, **a texture was allocated at the region's
full width and the bars inside it were cleared, drawn into, uploaded and sampled every frame** —
black nobody looks at, made and moved sixty times a second. At a 1920-wide window **66.6% of the
picture's texels were bars**; at 3440, 84%.

The manual already asked for the other thing: the picture is *"a region somebody may be capturing: a
capture that is neither the canvas nor a clean crop of it is worse than useless."* A region the
canvas's shape **is** a clean crop. The region was neither.

[ADR-0170](0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md) had already
taken this decision for a deck preview cell — *this aspect, as large as the box allows, centred* —
and recorded the alternative it rejected: fill the box and letterbox the texels inside it, *"which
is the thing `WHOLE_TEXTURE` refuses one level up"*. **The picture was the one level up, doing
exactly what the cell was forbidden.** This is that decision applied where it was first refused.

## Decision

**The picture is the canvas's shape, as large as its region allows, centred**, and the leftover is
the console's own ground rather than the engine's black inside a texture.

`fitted(inside, aspect)` is one function with two call sites the day it lands — the picture and a
preview cell. Its argument is an **aspect** and deliberately not a *canvas*: the picture's two
numbers are the canvas's and a cell's are `PREVIEW_ASPECT`'s, and the difference is recorded at that
constant. An audition renders the *same canvas*, so a non-16:9 canvas would letterbox inside a cell —
the second fit ADR-0170 rejected, one level down. The honest number for a cell is the canvas's too,
and it is not that yet for a structural reason: `View::draw` derives the cells itself where it is
*handed* the picture's rectangle, so making a cell canvas-aware means putting a canvas on `View` and
writing it per frame. Recorded at the constant as its own pass.

### The aspect arrives as the canvas's two numbers, not a ratio

`picture_rect(layout, canvas: (u32, u32))`. It is the shape `letterbox(canvas, target)` and
`Present::size` already speak, so a caller hands over the number it built its `Present` from rather
than deriving a float on the way in — and a ratio derived at a call site is where `9.0/16.0` gets
written for `16.0/9.0`, which is a bug nothing on screen shows. It carries its provenance: `(1280,
720)` reads as a canvas and `1.7777778` reads as a number somebody typed. The example threads it
from `Present::size()` rather than from its own constant, because **the shape the console draws and
the canvas `Present::draw` fits from cannot come from two places.**

### The extent is rounded to whole pixels, and the reason is not that a test passes

**The rectangle and the texture must be the same size, or the picture is resampled instead of
blitted.** A strict fit at the mock's narrowest gives 465.7778; `physical` rounds the texture to
466; so the console would allocate a 466-texel texture and draw it into a 465.7778-wide box, every
texel a fractional sample of its neighbours — in the one region on this panel that is a preview of
what is being captured, and buying nothing, because the texture's own ratio is 466:262 either way.
**The fractional quarter-pixel is not more faithful to 16:9; it is the same texture, softened.**

That the existing assertions then pass unedited is true and is **not** the argument. It was offered
as one and rejected: a rounding rule invented to make one assertion pass is the shape this
repository stops on.

The cost is stated where the rule is: the ratio is then the mock's 1.778626 rather than exactly
16:9, and `Present::draw` absorbs the difference — **sub-texel rather than redundant**, which is why
it stays. The bars inside the picture's texture go from 225 texels down each side at a 1440-wide
window to 0.111.

**The position is not rounded**, deliberately. Rounding it leaves a cell no longer centred in its
track, and centred is the other half of ADR-0170's rule.

### The leftover draws nothing, and it costs a capture

The bay's card shows through, as it does in every other empty body — the crate's own rule, and also
the mock's answer read carefully: `.program-view` **is** the picture, and what surrounds it is
`.program-body`, which sets no background of its own.

**What that costs is real and is written in three places a reader meets it.** After a solo the
window *is* this region, so an operator capturing a window whose shape is not the canvas's gets
`--c-panel` bars — a UI colour, and a light one in the Day room — where black would read as an
ordinary letterbox. Before this rule those bars were the engine's clear inside the texture and were
black. `karakuri-cli`'s `a` (`Live::snap_to_canvas`) is the answer one window along, written for
exactly this reason, and **the console's window has no `a` yet: this rule is what creates that
gap.**

## What was wrong, and is corrected rather than left

**`.program-view` at 466 wide is 262.125 tall, and the arrangement transcribed 262.** So the
Program bay's height was a rounded 16:9 from the day it was derived, and two sentences said
otherwise — `picture_rect`'s *"exactly 466 x 262, which is 16:9"* and `lib.rs`'s *"466 at 16:9 is
262"*. Neither was ever true; both are corrected under
[P-0066](../principles/0066-an-adr-is-a-description-of-history-corrected-but-never-revised.md)'s
test for a fact that was never true rather than an argument that changed.

**The test below already knew.** It checks the picture's ratio with a tolerance of `0.01` where
every other assertion in the file uses `1e-3` — 200 times looser, undocumented, and there because
466 x 262 is not 16:9. The assertion now says so.

**Two more sentences went stale with the change** and are corrected: `Engine`'s doc claimed
`Present::draw` is *"what makes the picture letterbox into its region in the first place"*, and the
reference-workload printout said the canvas was *"letterboxed into the picture's region"*. The
picture no longer has a region-shaped target.

## Consequences

- **What the texture stopped costing**, measured from the code as it landed: 49.1% of its texels at
  a 1440-wide window, 66.6% at 1920, 84.0% at 3440 — 0.45, 0.93 and 2.45 MiB per frame, and four
  times those figures on a 2x display. At the mock's narrowest nothing moves at all.
- **A widening window now remakes no texture**, where every frame of a horizontal drag used to
  destroy, rebuild and re-register one on the render thread. That is the second saving and it is an
  assertion rather than a sentence.
- **The rounding rule can only be held by a test stated in logical pixels.** `physical` rounds
  465.7778 back to 466, so the unrounded fit fails two logical assertions and passes every texture
  assertion — `mod gpu` is not a safety net for it.
- *Fill the region, as today* is caught by **four** tests across three consequences: the shape, the
  canvas's provenance, the bars in the texture, and the reallocation on a widening.
