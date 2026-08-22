---
id: 0065
title: Transport is two mechanisms, and closed form is the seam
status: accepted
date: 2026-08-03
supersedes: []
superseded_by: []
principles: [0032]
tags: [engine, ir]
---

# Transport is two mechanisms, and closed form is the seam

## Context

Transport was designed as one feature with three modes per slot: `free` (real time), `tempo` (rate
scaled by the room's tempo against a per-slot reference bpm), and `beat` (the clock locked to the
room's musical position, so it jumps and it reverses).

## Decision

**It is two mechanisms, and `closed_form` is the seam.** A closed-form clock can simply be *placed* —
one element pass, because the state does not depend on how it was reached. Accumulating material
integrates, and **reversing a sum is impossible rather than slow**, so all it can be given is a rate.

So `beat` is **refused for accumulating material**. Refused at the point the operator asks for it,
not at the point it would misbehave.

## The second refusal, which was not anticipated

Material that reads `beats` **already follows the room**. Scaling its clock by tempo as well makes it
follow **twice** — roughly the square of the tempo ratio. The check pass now records whether a
procedure reads `beats`, and `tempo` is refused for material that does.

Two controls, each correct in isolation, that compose into nonsense. Neither refusal is discoverable
from either control's own description.

The reference tempo is a **per-slot dial**, because **material has no intrinsic tempo** — a `.kir`
declares parameters and a capacity, not a bar length.

## Consequences

- **Scrub is on a key** (`u`/`i`, a quarter-beat per press) rather than waiting for a source that can
  reverse. Waiting would mean the reversing path never executes.
- Verified rather than asserted: rewound two beats, sixty steps, and **the frame matched the one drawn
  at that step on the way past, pixel for pixel** — same md5. Not a similar picture, the same picture,
  which is the whole reason `closed_form` exists. A control assertion is printed too: if the "before"
  frame were also identical, nothing moved and the test proves nothing.
- **A test corrected the repository's own examples.** A GPU test reported that `ring` is not closed
  form, because `age = age + dt` reads an emitted attribute. **Every ring in this repository claiming
  closed form was accumulating**, including the comment in an example committed an hour earlier. With
  no `spawn` block every element is alive from frame zero, so `age = t` is both correct and closed
  form.
- Four defects were injected and watched to fail: a seek that does not land, each refusal removed
  separately, and an inverted scrub sign — the last caught by a unit test and a record round trip
  rather than the GPU test, which is the layer it belongs to.

## Evidence

Session 2026-08-03T01:42Z–01:54Z, commit `9e40ea9`.
