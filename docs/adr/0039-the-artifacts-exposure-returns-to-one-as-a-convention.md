---
id: 0039
title: The artifact's exposure returns to 1.0, as a convention
status: accepted
date: 2026-07-31
supersedes: []
superseded_by: []
principles: [0018]
tags: [render]
---

# The artifact's exposure returns to 1.0, as a convention

## Context

With tone mapping in, `soft_points.kir` still carried `exposure 0.30`. Comparison renders of the
three candidate operators were close enough that **no choice was wrong**, so looking could not
decide it.

## Decision

ACES becomes the default operator, and the artifact's `exposure` goes back to **1.00**, decided
from the argument rather than the image:

> A mixer's fader means nothing if each Set arrives at a different nominal level.

`0.30` was a number **chosen to fight clipping**, and the tone mapper has taken that fight over.
What an artifact's `exposure` is *for* is saying how bright this material is, not standing in for
a missing stage — and **1.00 is the only value that means "nominal"**.

## Consequences

- L5's per-slot gain only becomes meaningful once every Set observes this. It is a convention
  across the library, not a property of any one artifact, which is what makes it worth writing
  down rather than fixing per file.
- The example's baseline changes on purpose; the tone-mapping tests were rewritten to assert both
  that the default is ACES **and** that it differs from Clamp, so "default" cannot silently become
  a no-op.

## Evidence

Session 2026-07-31T11:34Z. Follows
[ADR-0019](0019-exposure-is-three-things-and-none-stands-in-for-another.md).
