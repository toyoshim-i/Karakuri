---
id: 0247
title: One frame is rendered and scaled into each output
status: accepted
date: 2026-09-03
supersedes: []
superseded_by: []
principles: [0086]
tags: [engine, ui, operations]
---

# One frame is rendered and scaled into each output

## Context

[ADR-0246](0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md)
gave the render size to the output and named the question it left first: **whether two outputs at
two sizes cost two renders or one render and a scale.** Nothing in that record answered it, and
M5.6 meets it before it can draw a switchable list of anything.

The two readings were both defensible. *Render per output* is what the words most plainly say — an
output holds the size it is rendered at, so render it there. *One render, scaled* is what the code
already does: the deck's slot targets, the mix and the present pass are one canvas, and
`Present::draw` is handed two rectangles of different sizes a frame and letterboxes into each.

## Decision

**The frame is composited once and the present pass scales it into every output.** An output's size
is the size of the surface the frame is presented into, not a second place the scene is drawn.

**The single render is at the largest enabled output's size**, so that every output is a downscale
and none is ever upscaled. A downscale loses no framing and invents no detail; an upscale is
visible, and paying for it while a larger surface is already being fed would be paying for the
worse of the two pictures.

**What makes it acceptable is the same rule that made ADR-0246 acceptable.**
[P-0086](../principles/0086-a-procedure-knows-only-what-it-declares.md) — the render size is not
part of the picture — is why a frame drawn at one size and presented at another is the same picture
at two resolutions rather than two pictures. Without it, scaling would be an approximation and
rendering per output would be the only honest answer.

## Alternatives rejected

**Render once per output.** It is exact, it is the plain reading of ADR-0246, and it costs a full
scene per output: the deck's slot targets, every slot's draw and the mix, again. Two outputs
double the frame and four quadruple it, for a picture P-0086 says is the same picture. It buys
nothing the scale does not already give, and it buys it on the frame path, where the budget is.

**Render at the session's canvas and let each output scale either way.** Simplest, and it upscales:
a projector fullscreen on a 4K display would show a 1080p frame magnified because a default nobody
revisited said 1080p. The point of ADR-0246 was that the destination gets a say, and a scheme that
ignores the largest destination gives it none.

**Render at the largest output and let an operator cap it.** This is not rejected — it is the
same decision with the cap said out loud, and the cap is the operator's own size on an output,
which ADR-0246 already gives them. An operator who cannot afford 4K sets that output to 1080p and
the single render follows.

## Consequences

- **Adding a larger output raises what every frame costs**, because the single render follows the
  largest. That is visible and it is the operator's own act — enabling a sink — rather than
  something the instrument does behind them. The alternative was paying per output, which is worse
  and less legible.
- **`Present::draw` is already the mechanism** and needs no new one: it fits a canvas into a target
  of a different size and is handed several a frame today.
- **Nothing needs a second `Deck`, a second mix or a second present pipeline**, which is what
  *render per output* would have wanted and what M5.6 no longer has to design.
- **M5.6 has one fewer open question**, and what remains of *what names an output* is the identity
  itself — the destination, its size, and what tells a sink this repository owns from one a plugin
  brings.
- **The frame's single size is a derived number now**, not a setting: it is the maximum over the
  enabled outputs, and the session's `canvas` record is what it falls back to when no output says
  anything. Nothing is built by this record.
