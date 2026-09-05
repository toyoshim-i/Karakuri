---
id: 0034
title: The frame guard owns the encoder
status: accepted
date: 2026-07-30
supersedes: []
superseded_by: []
principles: [0093]
tags: [engine]
---

# The frame guard owns the encoder

## Context

The hot-swap brief claimed that a swap could not land mid-frame because the borrow checker
prevented it: `begin_frame()` returns `&mut Set` borrowed from `&mut self`, held for the frame
body.

**The claim was false, and a review demonstrated it** rather than arguing it — the command
encoder belongs to the caller and borrows nothing from `HotSwap`, so the borrow can end while
the encoder is still open and a second Set taken. The review recorded **two Sets of different
capacity into a single submit**.

The documentation was corrected to the weaker true statement, and the structural fix was
recorded as a requirement on M2: with one Set a convention suffices, but a deck compositing four
means one frame is built from four simulations.

## Decision

The **frame guard owns the encoder**. `Deck::begin_frame` returns a `Frame` holding both
`&mut Deck` and the `CommandEncoder`:

```rust
let mut frame = deck.begin_frame(&device, &queue);
frame.render(present.hdr_view(), steps);
present.draw(frame.encoder(), &surface_view);
frame.finish();
```

A second `begin_frame` while one is open is `E0499`. **A frame cannot be built from two
generations of Sets**, structurally.

`Deck::slot` deliberately returns a shared reference. Handing out `&mut HotSwap` would put
`HotSwap::begin_frame` back within a caller's reach and reopen the hole the guard closes.

The timing was deliberate: **tens of lines now, every call site later.** It was written into the
roadmap at decision time as "do this before there is more than one Set" and paid on schedule.

## Consequences

- **The negative test has a positive twin.** A `compile_fail` doctest can pass for an unrelated
  reason — `wgpu` missing from the doctest scope, say — so beside it sits a `no_run` twin
  differing by exactly the second borrow. The negative cannot pass spuriously without the
  positive failing too. And `E0499` was verified to be the *sole* error, not merely one of them.
- What the guard does **not** do is written down beside what it does: other passes may still
  join the frame's encoder (that is the point), an early return **ends** a frame rather than
  cancelling it (drop submits), and it says nothing about two separate `Deck`s.
- `HotSwap::begin_frame` keeps its weaker conventional property and now says so and points here.

## Evidence

Session 2026-07-30T12:52Z, 2026-07-30T19:16Z. Standing rule:
[P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md).
