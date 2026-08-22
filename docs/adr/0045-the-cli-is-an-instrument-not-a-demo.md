---
id: 0045
title: The CLI is an instrument, not a demo
status: accepted
date: 2026-07-31
supersedes: []
superseded_by: []
principles: [0030]
tags: [ui, process]
---

# The CLI is an instrument, not a demo

## Context

The deck existed in the engine and nothing could drive it. The brief for wiring it up carried two
sentences that decided the shape of the result:

> A window that opens and cannot be touched is a demo, not a tool.

> There is no on-screen UI, so **every action prints what it did**. An instrument that goes silent
> is unusable.

## Decision

Keys drive the deck live — focus a slot, take it on or off air, gain, exposure, **cycle the tone
mapper on running material**, status, help, quit — and each one prints its effect.

**Cycling the operator on moving material is information a comparison render cannot give.** Placing
ACES and AgX side by side as stills and switching between them at 60 fps on the same footage are
different experiences, and only one of them is how the choice is actually made.

## Consequences

- **Driving it is what found the off-by-one.** The digits were first bound `1`–`4`, so pressing `1`
  printed "focus slot 0" — inconsistent with the status line and every swap message. Reading the
  code would not have surfaced it; pressing the key did.
- Observations that only exist because someone could press a key: `space` on slot 1 prints
  `slot 1 off air — allocated, holding t 2.97s`, the status line then holds `t3.0s` for three
  seconds while slot 0 runs on to 5.6 s, and returning prints `resuming at t 2.97s`. **Allocated
  resuming rather than restarting became externally visible** rather than merely implemented.
- Under `--watch` with two slots, editing slot 1's L1 resets that slot's `t` and leaves slot 0
  untouched; a broken edit to slot 0 prints a diagnostic and `slot 0 unchanged; its Set is still
  running`, with `t` not blinking. Isolation demonstrated rather than asserted.
- `--render` and `--seq` render **the mix**, because the mix is what the window shows and there
  should be one definition of "the output". A per-slot dump is a different feature — it is M2's
  live preview, and it wants a default renderer per topology to be worth having.
- Unknown `-`-prefixed arguments now fail loudly; `--wtach` used to produce a "file not found"
  blaming the filename.

## Evidence

Session 2026-07-31T12:42Z–13:27Z. Standing rule:
[P-0030](../principles/0030-an-instrument-says-what-it-did.md).
