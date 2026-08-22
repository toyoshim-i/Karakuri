---
id: 0118
title: The built-in camera is a node, unconditionally and last
status: accepted
date: 2026-08-21
supersedes: []
superseded_by: []
principles: []
tags: [ir, engine]
---

# The built-in camera is a node, unconditionally and last

## Context

Several cameras in one Set, reached through a typed slot: `uses view : Camera`, with
`--edge renderer.view=…` naming which.

## Decision

**The built-in orbit becomes a node unconditionally, placed last** — cameras are the L3 procedures
followed by the orbit.

My brief said it should become a node **only when there is no L3 procedure**. The agent built exactly
that, **drove it end to end, and found it breaks**: with an L3 present, a renderer **cannot be bound
to the orbit**, because a thing that is not a node has no name — and `Record::Camera`'s `index` could
then only ever be `0`, which kills the second decision outright.

**Read literally, my decisions one and two were incompatible.** The agent chose the reading that keeps
the second alive rather than the one I wrote.

Placing it last means `L3:0` is the first L3 procedure where one exists, so **no existing address
moves**, while a Set with no procedure points at its own orbit with `L3:0`. The cost is one extra
camera node in a Set that has procedures — measured at 48 B + 16 B and one single-invocation dispatch
per frame.

## Consequences

- **The generated WGSL did not change by one character**, and that was a consequence of the notation
  rather than luck. `view.clip`, `.eye` and `.ray` resolve to `Ambient::Camera`, `Eye` and `Ray` —
  ambients that already exist — so **which camera a renderer reads is decided by which bind group the
  Set hands it**, and `karakuri-codegen` needed no edit at all.
- **Deferring `.target`, `.up`, `.fov_y`, `.near` and `.far` is what kept that boundary still.** Those
  are **a new capability rather than a new spelling** — a renderer cannot read them today — and opening
  them would need a new path carrying an L3's values to an L4, moving codegen and the uniform layout.
  The line between *spelling* and *capability* turned out to be exactly the line between *the boundary
  holds* and *it moves*.
- **A brief error of mine, and an instructive one.** I said the builtin-name collision refusal should
  also apply to a Camera slot. It does not and should not: that rule was narrowed to `Field` because
  only a **called** slot can collide with `sin(x)`, and a Camera slot is read as `sin.clip`. What I was
  adding *for consistency* would have broken it.

## Evidence

Session 2026-08-21T06:28Z–07:30Z, commit `356fd33`.
