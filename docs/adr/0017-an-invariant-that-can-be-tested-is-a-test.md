---
id: 0017
title: An invariant that can be tested is a test, not a sentence
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: [0016]
tags: [process, engine]
---

# An invariant that can be tested is a test, not a sentence

## Context

The project's invariants were prose in a `README`. Prose does not fail.

## Decision

Where an invariant can be mechanically checked, it is checked.

- **`no_clock_access.rs`** scans `karakuri-signal`'s **own source** for `Instant::now` and
  friends, and fails if one appears. "Rendering reads only the local oscillator" stopped
  being a claim about the crate and became a property of it.
- **The vertical slice is verified by offscreen render and readback, not by a screenshot.**
  Elements appear; additive blending accumulates; the same seed and steps reproduce bit for
  bit; a different seed differs; one step of `2·dt` lands where two steps of `dt` land. A
  screenshot proves it ran once on one machine; these fail when someone breaks them.
- **Generated WGSL is compiled by naga in a test.** A generator that has never been through a
  compiler emits plausible invalid code — and did, twice in one day.

## Alternatives rejected

- **Assert the invariants in documentation and rely on review.** Everything above was found
  by a test that a reviewer had already read past.

## Consequences

- A caveat learned the same day: **hand-built fixtures dodge bugs by accident.** The naga
  tests passed while the generator was broken, because the fixtures named their locals `uu`
  and `vv` instead of `u` and `v` as the specification's example does. See
  [ADR-0014](0014-generated-code-cannot-be-captured-by-a-name-a-procedure-can-spell.md).
  A fixture should be the real text, or be adversarial on purpose.

## Evidence

Session 2026-07-25T12:56Z, 2026-07-25T13:22Z, 2026-07-25T14:37Z. Standing rule:
[P-0016](../principles/0016-an-invariant-that-can-be-tested-is-tested.md).
