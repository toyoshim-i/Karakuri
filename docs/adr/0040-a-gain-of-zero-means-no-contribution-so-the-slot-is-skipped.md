---
id: 0040
title: A gain of zero means no contribution, so the slot is skipped
status: accepted
date: 2026-07-31
supersedes: []
superseded_by: []
principles: [0027]
tags: [engine, render]
---

# A gain of zero means no contribution, so the slot is skipped

## Context

A review's scratch probe failed with 47670 NaN channels in the mix, from a slot whose fader was
at zero.

**A gain of `0.0` does not mean "does not contribute".** `0.0 * NaN` is `NaN` and `0.0 * inf` is
`NaN`. With additive compositing **one NaN spreads across the whole screen**, and it wiped out the
surviving slot entirely — while the same slot taken `Allocated` was exact.

This is not hypothetical. Additive blending at these element counts diverges easily, and a
generator writes `sqrt` of a negative, `normalize` of a zero vector or a division by a parameter
without trying. Such an L4 parses, type-checks, costs, generates and runs; **nothing anywhere
rejects it.**

And for a VJ, pulling a fader to zero is **the last resort for escaping broken material**. It did
not work.

## Decision

A slot is included in the sum only when `residency == Live && weight != 0.0`. Zero weight is
**skipped**, not multiplied.

## Alternatives rejected

- **Sanitise NaN in the composite.** Hides a broken procedure instead of letting the operator
  escape it, and costs a comparison per texel per slot forever.

## Consequences

- Output is unchanged for finite values, since `acc + 0.0*x == acc`, so the existing gain tests
  were unaffected — which is also why they had never caught this.
- **The reasoning was already written down in the file that got it wrong.** `composite.wgsl`'s own
  header argues that skipping is required because *"`0.0 * x` is only zero for finite `x`"*. The
  author applied that argument to residency and not to the weight.
- My own test requirement — "a slot at gain 0.0 must not contribute, exactly" — was written and
  verified against **well-behaved material only**, so it passed. A test of a numerical guarantee
  needs a hostile input, not a typical one.

## Evidence

Session 2026-07-31T11:37Z–12:18Z. Standing rule:
[P-0027](../principles/0027-a-silently-wrong-image-loses-to-a-loud-failure.md).
