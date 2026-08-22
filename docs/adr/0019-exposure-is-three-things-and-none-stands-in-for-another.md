---
id: 0019
title: Exposure is three things, and none of them stands in for another
status: accepted
date: 2026-07-26
supersedes: []
superseded_by: []
principles: [0018]
tags: [engine, render]
---

# Exposure is three things, and none of them stands in for another

## Context

Three procedures were generated from one-line prompts by agents given the specification and
nothing else. Two compiled on the first attempt. **All three looked wrong** — flat saturated
slabs. Lowering the exposure alone, changing not one line of the procedures, produced what had
been asked for. The code was right the whole time.

## Decision

Three different things are called exposure, and each has an owner:

| | What it is | Who sets it |
| --- | --- | --- |
| An artifact's `exposure` param | How brightly *this material* glows. Part of the look | Whoever authors the procedure |
| The tone map | The transform from unbounded linear HDR to a displayable range | Automatic; derivable from the content |
| L5's per-Set gain | How loud this Set is against the others and against the room | The operator, live, semi-automatic from measurement |

**The tone map runs once, after the mix, immediately before output** — not per Set. A Set that
tone-maps itself hands an already-compressed image to the mixer, and compositing compressed
images muddies. This is the same rule the specification already had for sRGB: encode once, at
the end. So L5's per-Set exposure is a **linear gain applied before the mix**, and is not a
tone mapper.

## Alternatives rejected

- **Let the artifact's `exposure` cover for the missing tone mapper.** It is what was
  happening, and it does not merely delay the fix — **it breaks L5.** A mixer fader assumes
  every Set arrives at a sane nominal level. One Set authored at 0.05 for 262144 elements and
  another at 1.6 for 16384 makes the fader mean something different per Set, and the artifacts
  stop being portable.

## Consequences

- **The tone mapper is reclassified from a finish to a blocker.** It was scheduled with bloom;
  it moves ahead of it, because whether generation produces usable output is decided there.
- The per-Set measurement this needs — reducing a mip chain or one luminance histogram pass —
  is the **same plumbing** the budget governor needs for per-Set timing. One measurement hook,
  two readings.
- It also prevents the flash when a crossfade joins two Sets at different levels.

## Evidence

Session 2026-07-26T02:20Z–02:24Z. Standing rule:
[P-0018](../principles/0018-a-workaround-in-prose-is-a-missing-feature.md).
