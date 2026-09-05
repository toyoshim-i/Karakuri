---
id: 0115
title: Split work by file, not by phase
status: accepted
date: 2026-08-20
supersedes: []
superseded_by: []
principles: [0017]
tags: [process]
---

# Split work by file, not by phase

## Context

Two hours produced one commit. The cause was not measurement and not waiting for tests.

## Decision

**It was how I divided the work.** One agent was handed *commit 1, commit 2, commit 3* — and all
three touch `karakuri-ir` and `docs/ir-spec.md`, so they could only be done in order. **Dividing by
phase made it serial.**

Dividing by file would not have:

| Piece | Touches | |
| --- | --- | --- |
| Stop stage 4 claiming the figure | `ir/{cost,typed}.rs`, `cli/compile.rs` | |
| Make the engine report it | `engine/node/*.rs`, `engine/set.rs` | could have run at the same time |
| Verify offsets against naga | `codegen/tests/naga_test.rs` | could have run at the same time |

Only `docs/ir-spec.md` is shared, and that could have been merged at the end. **Three in parallel,
each committable as it finished.**

## Consequences

- Applied immediately: the next independent fix was picked precisely because it was *a different file
  in a different crate*, so it could not collide with the review already running.
- This sharpens `docs/contributing.md` §3 (P-0017 until it was retired on 2026-09-05): cutting the seam
  first is what makes parallel work possible, and **cutting it along the wrong axis makes parallel
  work impossible even when the seam is clean.**

## Evidence

Session 2026-08-20T18:07Z.
