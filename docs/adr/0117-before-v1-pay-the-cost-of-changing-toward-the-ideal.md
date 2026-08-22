---
id: 0117
title: Before v1, pay the cost of changing toward the ideal
status: accepted
date: 2026-08-21
supersedes: []
superseded_by: []
principles: [0058]
tags: [process]
---

# Before v1, pay the cost of changing toward the ideal

## Context

Making the built-in camera an ordinary node shifts every address after it. I had put that forward as
a reason for caution.

## Decision

> It is still in development, so a specification change costs nothing in compatibility. Until v1 is
> settled, pay the cost of changing toward whatever ideal you find.

**Compatibility and rework are a bill, not an argument.** Estimate the cost honestly when presenting
a fork — and **do not let the cost decide the recommendation.** The rule expires at v1, at which
point the bill starts being an argument.

## Consequences

- Applied the same day: `field(p)` was **deleted rather than kept as a compatibility alias**, which
  under the old habit would have been the safe choice and would have left the language carrying two
  spellings for one thing forever.
- It also reframes what a "risky" refactor is here. The real danger in making the camera a node was
  never the visible address shift — it was **`slot_of` and `params` disagreeing**: grow the origin by
  one without growing the parameter list at the same position and every renderer's address shifts
  **silently**, because the neighbouring node genuinely exists. That went into the brief as the thing
  to test by name, and it exists as a test.

## Evidence

Session 2026-08-21T06:26Z–06:28Z. Standing rule:
[P-0058](../principles/0058-before-v1-compatibility-is-a-bill-not-an-argument.md).
