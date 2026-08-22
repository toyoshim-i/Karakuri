---
id: 0071
title: There is no panic key
status: accepted
date: 2026-08-08
supersedes: []
superseded_by: []
principles: [0039]
tags: [ui, engine]
---

# There is no panic key

## Context

The roadmap placed a panic key in M2 — *"a panic key to a known-good Set"* — as part of surviving an
hour on stage.

## Decision

**It is not built, and the reasons are recorded** so the entry does not simply reappear.

**It adds no capability.** Everything it would do is already recoverable by hand: `space` takes a slot
down, `\` returns gain, `` ` `` returns exposure. All it adds is *all of it, everywhere, without
choosing*.

**And that is the objection.** An anomaly is **usually one frame and usually harmless**, so a control
that resets the performance in response to one does **more damage than the thing it responded to**.
Worse, a panic key learned to be sometimes wrong becomes a control the operator hesitates over — and
hesitation is the one property a panic key must not have.

**It also could not do its stated job.** *"Known-good Set"* implies a rollback target, and
`HotSwap::previous` exists **only inside the trial window** and is retired the moment a build is
accepted. A permanent rollback target means holding a Set's worth of VRAM per slot for ever, which is
the VRAM budget that was never built.

Worth recording: my first objection to `previous` was that it might not be safe. **That was wrong.
The real reason is that it does not exist.**

## What was built instead

The measurable failure underneath: **a single NaN texel destroyed a slot's entire level reading.**
Non-finite luminance is now counted and excluded from mean and peak, with the excluded texels **kept
in the denominator** — treated as black — because otherwise one texel makes two slots incomparable.

`bad_texels` is shown and **is deliberately not a fault indicator**. No threshold, no flag, nothing
saying "broken". Being commonplace and harmless is precisely what makes it a poor proxy for *the
material is wrong*, and **a warning that fires on healthy material teaches the operator to ignore
warnings.**

This milestone reached that same answer three times: the tempo octave, semi-automatic gain, and here.

## Consequences

- Of eleven review findings, **one was a real bug I had introduced**: with every texel non-finite,
  `peak` printed its sentinel `-3.4e38`, putting 41 characters into a status line whose whole design
  is aligned columns readable in the dark — broken in exactly the way this change existed to prevent.
- A portability finding that matters beyond this file: the finiteness test was written as a
  comparison, and **WGSL explicitly permits an implementation to assume NaN does not occur**. Metal's
  default fast-math folds such a comparison to `true`, so NaN would silently rejoin the sum on a
  shipping backend. Rewritten as a bitcast of the exponent — integer work, which float optimisation
  cannot touch.

## Evidence

Session 2026-08-08T18:09Z–19:35Z, commit `15dec5a`. Standing rules:
[P-0039](../principles/0039-a-warning-that-fires-on-healthy-material-teaches-the-operator-to-ignore-warnings.md),
[P-0040](../principles/0040-a-check-the-compiler-may-assume-away-is-not-a-check.md).
