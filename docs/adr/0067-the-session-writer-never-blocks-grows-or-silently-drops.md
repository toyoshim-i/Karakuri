---
id: 0067
title: The session writer never blocks, never grows, and never silently drops
status: accepted
date: 2026-08-03
supersedes: []
superseded_by: []
principles: [0091]
tags: [store, engine, determinism]
---

# The session writer never blocks, never grows, and never silently drops

## Context

Writing the session stream is on the frame path, and the frame path may not allocate. Replaying it
back is what turns *"the Set can be rebuilt"* into *"the performance can be replayed."*

## Decision

**The render thread writes nothing.** `Line::new` stringifies immediately, so a frame that builds a
record has allocated. Instead the frame **moves records into a `Vec` that already has capacity**, and
serialisation and I/O belong to the writer thread. Whole buffers are handed over and empty ones come
back, so the only allocation is at startup.

**What happens when the writer falls behind is the design, not an edge case.** The channel is bounded;
a frame that finds it full **does not block, does not grow the buffer, and does not silently skip** —
it **counts what it dropped and says so at exit**. A session with holes in it is not a session.

`audio` was left out at first and the reason recorded in the source: `Record::Audio` owns a `Vec`, so
cloning it every frame is exactly the allocation the whole arrangement avoids.

## Consequences

- **`tick` finally got a writer here**, which is what closed
  [ADR-0063](0063-an-invariant-that-is-not-yet-true-says-so.md). Two replays of one session produce
  byte-identical frames.
- `audio` landed the same day, by **swapping rather than cloning**: the recorder hands
  over an empty shell and takes the filled record, so the band buffer moves, and the writer returns
  the buffer after serialising so the shells circulate like the batches.
- **The shell count was got wrong, and the mistake is instructive.** Three looked sufficient, since at
  most one is in flight per frame. But **a shell does not return until the batch containing it is
  written, and a batch is not handed over until it is full** — with almost nothing in a batch, the
  batch never fills, so the shells never come back: **509 of 512 frames went unrecorded.** The
  inventory has to cover *everything in flight*, not one frame's worth.
- **Two kinds of loss are counted separately.** A lost batch is a second of everything; a lost audio
  frame is one measurement, and it shows up not as a gap but as **a value the bus invents** on replay.
  Merged into one number, the operator cannot tell which they have.

## Evidence

Session 2026-08-03T04:00Z (`12c4d66`) and 2026-08-03T14:12Z (`768b4d3`).
