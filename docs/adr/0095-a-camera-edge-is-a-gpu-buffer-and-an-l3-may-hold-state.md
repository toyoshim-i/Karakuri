---
id: 0095
title: A Camera edge is a GPU buffer, and an L3 may hold state
status: accepted
date: 2026-08-16
supersedes: []
superseded_by: []
principles: [0032]
tags: [ir, engine, render]
---

# A Camera edge is a GPU buffer, and an L3 may hold state

## Context

Two examples of the dynamism expected from L3 — *follow the first vertex*, and *jump about on the
beat while staying pointed at the centre* — **falsified two sentences written into the
specification two days earlier**, and did so usefully.

## What broke

> **An L3 node has no geometry input and no GPU pass.** … It is therefore the first node whose
> evaluation is host-side.

Both false. A camera following a vertex **takes Geometry as an input** — an edge the algebra's
`L3 : () -> Camera` forbids — and the value it wants is in a GPU buffer. And it **cannot be read
back**: `Set::read_elements` and `live_count` say of themselves that they are a stall and must never
be called on the frame path. A GPU-to-CPU read of one element per frame is the one thing this
architecture exists to avoid.

## Decision

**The Camera edge is a GPU buffer. Which producer writes it depends on the L3.**

- An L3 of time alone — an orbit, a beat-driven jump — is **written by the host** with
  `queue.write_buffer`. Today's `Orbit` works unchanged.
- An L3 that reads geometry is **written by a small compute pass**. No readback.

One consumer, two producers. The earlier decision that **camera state crosses the edge, not a
matrix**, survives — it only moves. The reason for it stands too: **an interpolation of two
view-projection matrices is not a projection of anything**, so a node emits the six numbers that
*are* a camera and the engine derives `view_proj`, `basis` and `depth_range` in the one place that
already keeps them from drifting.

**And an L3 may hold state, where an L2 may not.** Three of L2's four reasons do not apply —
L3-multiple is a blend of orbits rather than a chain, six numbers warm instantly, compaction is
irrelevant. Only `closed_form` applies, and it is already handled by an existing rule: **a Set is
seekable when every node in it is.** That matters because the craft of camera work is almost
entirely **smoothing**, and smoothing is state; rigid following is unwatchable.

## Consequences

- **A coupling the layer algebra could never have predicted: `near` and `far` were decoration until
  yesterday.** They described a frustum nothing measured, until the weighted blend's weight function
  began normalising against it. An L3 that moves `far` every frame now moves **every weighted
  fragment's weight** every frame.

## Evidence

Session 2026-08-16T04:42Z–04:52Z.
