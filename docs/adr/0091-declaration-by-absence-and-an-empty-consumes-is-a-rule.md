---
id: 0091
title: Declaration by absence, and an empty `consumes` is a rule
status: accepted
date: 2026-08-15
supersedes: []
superseded_by: []
principles: []
tags: [ir, engine]
---

# Declaration by absence, and an empty `consumes` is a rule

## Context

A renderer now has three shapes — sprites, lines, and the whole frame — and each needs to be
declared.

## Decision

**By absence and presence, never by a header field.** An L4 draws lines by assigning `clip_b`; it
draws the whole frame by **having no `vertex` block**. There is exactly one place the fact can live,
so it cannot disagree with itself. `Checked::topology` is *declared* on an L1 and *inferred* on an L4.

**And `consumes` must be empty for a fullscreen renderer — as a rule, not as a consequence.** It
follows anyway, and making it a checked rule is what makes **skipping the paired L1's simulation
entirely** *provable*. Left implicit, that skip would be an optimisation leaning on an interpretation
of the language.

`eye` and `ray` are **given**, not derived. Handing each shader an inverse camera matrix to build its
own ray would duplicate the projection convention into every marching shader, and a small divergence
produces a picture that looks nearly right. The camera is built in, so the convention belongs to the
engine.

## Consequences

- Verified rather than asserted: an L1 with a `spawn` block paired with a fullscreen L4 shows **live
  count 0 after four steps**, against a sprite-L4 control where it grows. `t` still advances, because
  the marcher reads it. Eight injected defects each failed the test named for them — including the
  naive "skip inside `prepare`", which stops the clock too.
- **The language ran ahead of the engine, deliberately and briefly.** The grammar accepted a
  fullscreen L4 before the draw path existed, and `Set::build` refused it with a hint saying exactly
  that; the MCP vocabulary carried the same limitation so a model would not meet an unpredictable
  refusal. That is the state the eight SDF builtins have been in since M1. The line drawn: **an hour
  is acceptable, spanning a milestone is not.**

## Evidence

Session 2026-08-15T18:57Z–19:13Z.
