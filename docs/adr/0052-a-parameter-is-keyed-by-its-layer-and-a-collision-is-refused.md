---
id: 0052
title: A parameter is keyed by its layer, and a collision is refused meanwhile
status: accepted
date: 2026-08-01
supersedes: []
superseded_by: []
principles: []
tags: [engine, format]
---

# A parameter is keyed by its layer, and a collision is refused meanwhile

## Context

Found by a review, and it predates bindings entirely. `Set::build` collected `l1.params` chained
with `l4.params` into a **flat `HashMap<String, f32>`**. When both procedures declare `exposure`,
**L4's default silently overwrites L1's** — so an L1 shader runs at a number its own `.kir` never
states, before any binding is involved.

`Record::Bind` and `Record::Param` both carry a `layer`. The engine did not.

## Decision

The right shape is to **key values by `(layer, name)`**, matching what the record format already
does. That changes a public type and touches the CLI and several tests, so the review **described
it rather than doing it** — the correct scope call.

What landed is the stopgap, chosen to be honest about being one: `Set::build` returns
`SetError::ParamCollision`, naming **every** colliding parameter at once. One severity, regenerate
rather than proceed.

**It is correct and it is not the answer.** Two procedures both wanting a `hue` is not an error, and
refusing it is a restriction the design does not intend. Recorded in the roadmap as such, so the
stopgap does not calcify into the specification by being what the code does.

## Consequences

- No example pair collides, so nothing that worked stopped working — which is also why this survived
  so long unnoticed.
- The general shape: **a silent wrong value becomes a loud refusal first, and the real fix follows.**
  Refusing is cheap and stops the damage; changing the key is a public type change and can wait for
  its own slice. What must not happen is stopping at the refusal and calling it the design.

## Evidence

Session 2026-08-01T05:20Z–05:31Z.
