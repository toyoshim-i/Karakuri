---
id: 0078
title: A frame that is discarded must not already have been recorded
status: accepted
date: 2026-08-11
supersedes: []
superseded_by: []
principles: [0028]
tags: [determinism, engine]
---

# A frame that is discarded must not already have been recorded

## Context

Looking at the window output for the first time as a deliverable found four things, and one of them
was a hole in the invariant this project has been defending all along.

The order was:

```rust
let steps = self.steps();
recorder.push(Record::Tick { steps });   // recorded
self.measure_audio(steps);               // audio recorded, signals advanced

let surface_frame = match self.surface.get_current_texture() {
    Err(Lost | Outdated) => { configure; return; }   // frame discarded here
```

The `tick` is written **before** the frame can be thrown away, so the deck does not advance while
the session says it did. On replay the `tick` is read and the deck *does* advance, so **live and
replay diverge by every dropped frame** — and `Outdated` happens ordinarily, on a resize or a
display reconfiguration.

The same shape as the beat tap that built a `Record::Tempo` and dropped it
([ADR-0073](0073-a-control-surface-is-a-test-of-the-invariant.md)).

## Decision

**Acquire the surface first, then record.** The fix is ordering alone, **and it is also more
correct**: not calling `steps()` leaves `self.last` where it was, so the discarded frame's time
carries into the next one. As it stood, that time was recorded, never simulated, and lost.

## Consequences

- Two smaller findings from the same pass, both about silence: `Err(_) => return` was swallowing
  `OutOfMemory` and `Other` for ever — the window freezes and nothing is said — so those are named
  once, while `Timeout` stays silent correctly.
- One suspicion checked and dismissed rather than assumed: with alpha now carrying coverage, an
  `alpha_modes[0]` that is a transparency mode could make the window see-through. It cannot —
  `present.wgsl` returns `vec4(tonemap_apply(c), 1.0)`, opaque under every mode.

## Evidence

Session 2026-08-11T09:06Z.
