---
id: 0165
title: The repaint decision is one closed list, never an operation's return value
status: accepted
date: 2026-08-24
supersedes: []
superseded_by: []
principles: [0072]
tags: [ui, performance]
---

# The repaint decision is one closed list, never an operation's return value

## Context

[P-0072](../principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)'s
first clause says a panel with nothing changing on it does no per-frame work. Implementing it
means the loop stops drawing unless something asks, and **the failure that invites is silent**: a
change reaches the model, nothing asks for a frame, and a control goes on showing a value that is
no longer true. Nothing crashes and nothing logs. An over-repainting panel wastes a budget; an
under-repainting one lies.

There was already a route to that failure sitting in the code, and it was inviting.
`Panel::moved` returns an `Option` — `None` when a drag moved the boundary by less than
`WORTH_SAYING`, which is half a *logical* pixel and exists to keep the readout from printing sixty
identical lines a second. Deciding the repaint from that `Option` reads perfectly, is one
character shorter, and is wrong: half a logical pixel is a whole physical one on a 2× display, and
the mistake lands in the middle of the one gesture an operator is watching closely.

## Decision

**One closed list of everything that can change what the console shows, and the repaint is decided
from that.** `Change::{Pointer, Wheeled, Operated, Room, Viewport}`, with a single function from a
`Change` to a `Repaint`. Every handler in the window loop reaches it; none decides for itself.

**Adding something the panel reacts to is adding a variant, and the `match` does not compile until
its repaint is decided.** That is the whole reason it is an enum rather than a set of calls: the
failure mode here is omission, and omission is the one thing a closed list makes impossible.

**The decision is taken from what happened, never from what an operation returned.** An operation's
return value answers a question that operation was asked — was this worth printing, did anything
move far enough to mention — and reusing it as "did the screen change" couples a repaint to a
readout's threshold. Who claimed the event is the fact that matters, and it is the fact used.

**`egui`'s own repaint delay is honoured as a deadline, not collapsed to now.** It asks to be
repainted *after* a delay when it is animating — a blinking cursor, a fading tooltip — and
answering immediately turns an animation into a spin, which is the same bug this record exists to
prevent wearing the opposite sign. A delay of zero is different in kind and is treated as such: it
means *I have not finished drawing the frame you already asked for*, so it is not charged against a
still panel, where a named delay is.

It lives in `crates/karakuri-console/src/repaint.rs`, which knows no toolkit, for the reason the
model does: a `winit` handler cannot be called from a test, and there is nothing to assert against
inside an event loop.

## What it found

`egui`'s frame-level repaint request was read **nowhere at all**. The loop honoured
`EventResponse::repaint` for events it forwarded and never looked at the frame's own delay — which
was invisible while the loop spun at 60 Hz and would have become a dead animation the moment it
stopped. And `ScaleFactorChanged` asked for no frame of its own, relying on `egui`'s response and
on macOS's habit of sending a `Resized` after it.

## Alternatives

**Decide from `Panel::moved`'s `Option`.** Described above. Rejected, and recorded rather than
merely avoided, because it is what a reader reaches for first and the code makes it look correct.

**Leave the decision in the window loop, at each handler.** Where it was. Rejected on both halves
of the failure: it cannot be tested, and nothing tells the next person adding a handler that a
decision is owed there.

**Answer `egui`'s delay immediately.** One less state to carry. Rejected: it is a spin, and the
reading would show it as one.

**Measure over a fixed frame count, as before.** It was the right sample for a loop that always
drew; it cannot express *no frames at all*, which is now the claim being made. The reading is over
wall clock, and the three seconds are chosen because a 60 Hz spin fills them with the 180 frames
the old sample used — so the two numbers describe the same stretch of time.
