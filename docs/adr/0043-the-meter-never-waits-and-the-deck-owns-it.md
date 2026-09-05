---
id: 0043
title: The meter never waits, and the deck owns it
status: accepted
date: 2026-07-31
supersedes: []
superseded_by: []
principles: [0095]
tags: [engine, ui]
---

# The meter never waits, and the deck owns it

## Context

Setting a fader needs a number. Per-Set level measurement is also the plumbing the governor will
want, so it is one hook read two ways.

## Decision

Mean and peak **linear Rec.709 luminance** — not an RGB average, which would flatter green — over
the slot's **own** target, **before gain and opacity**. That is the level the material *arrives*
at, which is the input to setting a fader rather than the output of having set one. Mean is the
matching figure; peak is the warning, since a Set with an outlying peak dominates the mix whatever
the fader says.

**Never blocking is the binding constraint.** Reduce on the GPU, copy to a staging ring,
`map_async`, and **do not wait** — read whatever has arrived. It is therefore a few frames behind,
**and that is the specification, not the defect. A meter that stalls the frame is the bug.** The
ring holds four; a frame finding it full **skips rather than waits**, and the skips are counted so
the size is checkable.

**The deck owns it.** A bind group keeps its texture view alive, so a meter held outside the deck
would keep measuring the pre-resize target after `Deck::resize` reallocates — presenting as a
level that freezes at whatever was on screen when the window was dragged.

**`arm` is separate from `record`**, because `map_async` resolves against the submissions
outstanding **when it is called**: arming before the copy is submitted delivers a three-frame-old
buffer as this frame's.

**An Allocated slot reads `None`, not its last value.** Going off air drops the retained reading
*and* bumps a generation counter to invalidate measurements in flight, so a result recorded while
the slot was Live cannot arrive two frames later and resurrect a level.

## Alternatives rejected

- **Automatic gain.** Manual, as already settled: an exposure that moves on its own is the worst
  behaviour on a stage. The right order is to show the number first and decide later what should
  react to it.
- **A hook for a future automatic gain.** Explicitly not built. **A hook written for an
  anticipated design usually fixes the wrong design.**

## Consequences

- Opt-in via `Deck::enable_meters`. A caller that never calls it allocates nothing, so an
  offscreen `--render` pays zero.
- Lag is reported **with its conditions**, and the row that was not measured says so: 1 frame
  headless with per-frame `poll(Wait)` pacing; 19–126 (median 64) unpaced, with 218 of 240 frames
  skipped — the ring correctly declining to grow to cover a loop sixty frames ahead of the GPU;
  and 2–3 for a vsync'd window, **inferred from queue depth, not measured**, because measuring it
  needs a surface. The paced figure is stated as a lower bound rather than an expectation.
- **It showed something nobody asked it to.** Slot 1 (`spark_fountain`) reads mean 0.014 at 0.5 s,
  0.047 at 2.0 s, 0.067 at 4.0 s, then settles — the fountain filling up, in numbers. That is
  exactly the phenomenon Priming exists to hide ("fade one in and you watch the particles being
  born, which usually looks bad"). The answer to *what do you measure to say a Set is warm* arrived
  before the feature that needs it.

## Evidence

Session 2026-07-31T12:42Z–15:37Z.
