---
id: 0072
title: Auditioning adds a draw and never a step
status: accepted
date: 2026-08-10
supersedes: []
superseded_by: []
principles: [0041]
tags: [engine, ui]
---

# Auditioning adds a draw and never a step

## Context

Per-slot preview: `v` cycles the output between the mix and each slot. The whole feature reduced to
one rule.

## Decision

**Auditioning adds a draw. It never adds a step.**

If looking at a slot advanced it, the property every residency level rests on — `t` moves only through
`Set::prepare`, so a slot taken down and brought back resumes where it stopped — would break. And it
would break **in the least visible way possible**: you watch a moving picture, decide you like it, put
it on air, and it is not where you were looking.

**No second pass either.** The mix runs as it always does with the target slot at unity and the others
skipped, so `0.0 + 1.0*src` puts that slot's texels straight onto the target. The tone mapper, the
present pass and the readback are untouched.

**Faders are deliberately ignored.** What is being judged is the level the material *arrives* at, which
is the input to setting a fader rather than the result of having set one — the same order the meter
measures in. So the audited slot's meter runs too. **That is the difference between looking and
auditioning.**

## Consequences

- **Priming and preview meshed without knowing about each other.** A Set that has never stepped has no
  element state to draw, so peeking at a candidate in an off-air slot is black — and priming is exactly
  what exists for that. Take it down, warm it, look at it.
- **The third vacuous test of the milestone.** A meter test ran its slot Live first, leaving the
  previous picture in the target, so it passed with **both draws removed** — reading precisely the
  stale-value-as-current state that `Meters::retire` exists to prevent. Rewritten to warm with priming
  alone.
- **My stated reason for a behaviour was wrong.** I had written that a slot must be drawn every frame
  or it flickers; a slot's target **persists**, so not drawing leaves the previous picture and is
  indistinguishable at the output. The real reason is **resize**, the one moment the target is
  recreated. Reason corrected, and the test rewritten to hit that moment.
- One measurable visual bug from review: after a resize, an **Allocated** slot's audition draws at the
  old aspect ratio for ever, because the L4 uniform's viewport is written only by `prepare` and a
  parked slot never goes through it. 32417 of 131072 channels differed.
- Written down where the code is: the extra pass is **not in the budget**, and it falls inside the
  watchdog's judging window — so auditioning a heavy slot can roll back an unrelated slot's build.

## Evidence

Session 2026-08-10T22:42Z, commit `57a9519`. Standing rule:
[P-0041](../principles/0041-observing-must-not-advance-what-is-observed.md).
