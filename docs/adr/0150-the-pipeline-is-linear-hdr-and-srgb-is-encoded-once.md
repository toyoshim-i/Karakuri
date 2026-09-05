---
id: 0150
title: The pipeline is linear HDR, and sRGB is encoded once at final output
status: accepted
date: 2026-08-23
supersedes: []
superseded_by: []
principles: [0064]
tags: [engine, render, docs]
---

# The pipeline is linear HDR, and sRGB is encoded once at final output

## Context

This rule has been in force since the first committed specification and has never had a
record. It was stated in `README.md`, then lifted with the rest of the invariants into
`docs/invariants.md`, and it is the one rule in that document with no principle behind it —
every other line there is already a principle or already an ADR, which is what made
`invariants.md` a sixth restatement of rules that live elsewhere rather than a home for
anything ([ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md)).

Deleting `invariants.md` therefore needs this written down first, and writing it late is
worth doing rather than skipping: three later records lean on it without ever deciding it.
[ADR-0019](0019-exposure-is-three-things-and-none-stands-in-for-another.md) argues the tone
map runs once after the mix by analogy — *"the same rule the specification already had for
sRGB: encode once, at the end"* — and cites a rule with no address.
[ADR-0037](0037-tone-mapping-is-a-uniform-and-the-default-is-chosen-by-looking.md) puts tone
mapping in the present shader because that is where the single encode already was.
[ADR-0008](0008-blend-is-declared-now-so-the-second-mode-is-an-addition.md) accepts a second
blend mode on the ground that *"the existing `Rgba16Float` pipeline already accommodates"*
it.

The reconstruction that produced these records read the session history for decisions that
were argued. This one was never argued, because nobody proposed the alternative — which is
exactly the case a principle exists for, and exactly the case that gets lost.

## Decision

**Every intermediate target is `Rgba16Float`, linear, and unbounded. sRGB encoding happens
exactly once, at final output, on values a tone mapper has already brought into `[0, 1]`.**

The single encode is structural rather than a convention that has to be remembered.
`Present` owns both ends of it: the linear HDR target every `VideoSource` renders into, and
the surface it writes to. The transfer is the hardware's, from an `Rgba8UnormSrgb`
destination format, so no shader in the workspace spells linear-to-sRGB on a render target
at all. A second encode would require a second sRGB-format target, which is a visible thing
to review rather than a line buried in a fragment shader — this is
[P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md)
satisfied on the structural side.

Values above 1.0 are part of the contract and not an overflow to be clamped. They are the
input to the tone mapper and to bloom, and the reason additive compositing of four deck
slots composes at all.

Linear here means **Rec.709 primaries**, stated wherever colour is folded to a scalar:
`shaders/meter.wgsl` weights luminance `0.2126 / 0.7152 / 0.0722` rather than averaging
three channels, because green carries roughly ten times the luminance blue does at equal
magnitude and an operator matching faders on an averaged number is matching the wrong
quantity.

## Alternatives rejected

- **An 8-bit intermediate, `Rgba8Unorm` or `Rgba8UnormSrgb`, between passes.** Half the
  bandwidth and half the footprint, on a path where four deck slots each hold a
  full-resolution target. It loses because the headroom above 1.0 *is* the signal: an
  additive mixer receiving pre-clipped slots cannot recover what was clipped, the tone
  mapper downstream has nothing left to compress, and the difference shows first on exactly
  the material this project renders — dense additive point sprites that drive channels far
  past 1.0 unevenly ([ADR-0037](0037-tone-mapping-is-a-uniform-and-the-default-is-chosen-by-looking.md)).

- **Encode to sRGB at each pass boundary "so the intermediate looks right."** The reading is
  seductive when debugging, because a saved intermediate opens in an image viewer looking
  like the picture. It loses because every arithmetic operation after it is then wrong —
  blending, the meter's mean, the probe's readback — and nothing in the type system
  distinguishes a display-referred texture from a linear one. The failure is silent and
  plausible, which is the class [P-0027](../principles/0027-a-silently-wrong-image-loses-to-a-loud-failure.md)
  rules out.

- **A per-Set encode, so each `VideoSource` hands L5 something displayable.** Rejected on
  the same ground [ADR-0019](0019-exposure-is-three-things-and-none-stands-in-for-another.md)
  rejects a per-Set tone map: compositing already-compressed images muddies, and a mixer
  fader assumes every Set arrives at a comparable nominal level.

- **Leave it unrecorded, since nothing has ever violated it.** This is what was happening.
  Nothing has violated it because nothing has proposed to; the proposal that would — an
  8-bit intermediate, offered as a memory saving when four slots at 4K are resident — is
  the ordinary one, and a rule with no address is a rule that gets re-argued from scratch.

## Consequences

- The IR keeps `srgb_to_linear` and `linear_to_srgb` as builtins, and that is not a
  contradiction. They convert *authored colour data* — a hex value a person picked in a
  display-referred tool — into the linear space the pipeline works in. What is ruled out is
  a **render target** carrying the transfer function, not a procedure doing arithmetic with
  one.
- The offscreen path is bound by this too, and already obeys it: `--render` draws into the
  same `Rgba16Float` target and encodes once into an `Rgba8UnormSrgb` texture on write, so a
  PNG and the window are the same pipeline rather than two that agree.
- `crates/karakuri-engine/tests/tonemap.rs` renders through an `Rgba8UnormSrgb` target
  precisely so the hardware encode is inside what is asserted, which makes the second half
  of this rule tested rather than reviewed
  ([P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md)).
- Bloom is not written yet. When it is, it reads the HDR target before the present pass and
  is bound by this record rather than free to choose its own format.

## Evidence

`present.rs` and `shaders/present.wgsl` argue the one-call-site design in their own module
docs, and predate this record. Standing rule:
[P-0064](../principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md).
