---
id: 0042
title: A silently wrong image loses to a loud failure
status: accepted
date: 2026-07-31
supersedes: []
superseded_by: []
principles: [0094]
tags: [engine]
---

# A silently wrong image loses to a loud failure

## Context

The composite reads slot targets with `textureLoad`, and **an out-of-range load returns zero by
specification** — it does not fault. So resizing `Present` and forgetting the deck produces **a
black frame, every frame, with nothing logged anywhere.**

## Decision

`Frame::render` checks the sizes it was handed and **panics at the call site**. A crash naming the
mismatch is better than a correct-looking pipeline producing nothing.

The general form: where a hardware or API behaviour turns a programming error into *plausible
output*, the error is converted back into a failure at the place that caused it.

## And where a hole cannot be closed, it is written down

`mem::forget(frame)` defeats the frame guard, and this is **reported and documented rather than
hidden**. Forgetting a `Frame` ends the `&mut Deck` borrow, so a second `begin_frame` opens at
once and the encoder is dropped unsubmitted. That is worse than a lost frame:

- `Set::prepare` has already advanced `steps_taken`, and its `queue.write_buffer` calls land
  regardless — they are on the **queue**, not the encoder.
- `Set::render` has already flipped parity on the CPU, while the compute passes that justified the
  flip were discarded.

So `t`, parity and the GPU element buffers are **permanently out of step**, and L4 reads the wrong
buffer from then on. Not closable without redesigning `Set`, so it is named in the module's
"what this does not enforce" list.

Checked and found harmless in the same pass, and recorded as such: two separate `Deck`s, a `Frame`
stored in a struct (the borrow is still held), and a panic between `begin_frame` and `finish`
(unwinding runs `Drop`, which submits as far as it got).

## Evidence

Session 2026-07-31T12:18Z. Standing rule:
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md).
