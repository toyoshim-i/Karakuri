---
id: 0069
title: Blend modes are chosen by the pipeline, not by the vocabulary
status: accepted
date: 2026-08-08
supersedes: []
superseded_by: []
principles: [0090]
tags: [render, engine]
---

# Blend modes are chosen by the pipeline, not by the vocabulary

## Context

L5 needed blend modes. The obvious list is the VJ vocabulary: `screen`, `multiply`, `over`, `add`.

## Decision

**`add`, `over`, `max` — and the list was decided by the pipeline, not by what is familiar.**

`screen` is `d + s − d·s` and `multiply` is `d·s`. Both are **display-referred**, defined on `[0,1]`.
This mix is **unbounded linear HDR with the tone mapper still downstream**, so `screen(2.0, 2.0)` is
`0.0`. A familiar name that is a bug.

All three modes are one expression — `acc ← mix(acc, f(acc, gain·src), opacity)` — with `add`
hand-simplified so it is **bit-identical to before**.

**This is what finally made `gain` and `opacity` different things.** `gain` is the level the material
arrives at and touches colour only; `opacity` is the fader on the blend and is **the only side that
touches what a layer covers**. Under `add` they collapse into a single multiply — which is exactly
why one number had sufficed until now.

`over` required L4's **alpha to start carrying coverage**: colour stays additive while alpha
accumulates `1 − Π(1−aᵢ)`.

## Consequences

- **`Deck::set_opacity`'s debt was repaid without breaking the rule that produced it.** The rule is
  *no record for a control nobody can operate*; the resolution was ordering — the key came first
  (`;` / `'`), then the record (`opacity`, `blend`), then the status display and the replay path.
- Two of the five review findings were **created by my own fix**, which is the argument for
  implementation and review being separate agents rather than separate passes.

## Evidence

Session 2026-08-06T00:26Z–2026-08-08T17:51Z, commit `55016f2`.
