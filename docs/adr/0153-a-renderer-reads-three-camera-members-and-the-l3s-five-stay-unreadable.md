---
id: 0153
title: A renderer reads three camera members, and the L3's five stay unreadable
status: accepted
date: 2026-08-21
supersedes: []
superseded_by: []
principles: []
tags: [ir, render]
---

# A renderer reads three camera members, and the L3's five stay unreadable

## Context

A renderer had reached its camera through three words reserved language-wide — `camera`, `eye`,
`ray` — which is the shape that caps a procedure at one input.
[ADR-0152](0152-a-kir-names-a-slot-and-the-set-names-the-nodes.md)'s notation replaced them with a
declared slot: `uses view : Camera`, read `view.clip`, `view.eye`, `view.ray`, bound by an `edge`.

Turning a reserved word into a member of a named type raises the obvious question, because the
member list is now something a checker holds rather than something the language reserves: an L3
writes **six numbers** — `eye`, `target`, `up`, `fov_y`, `near`, `far`, the state
[ADR-0095](0095-a-camera-edge-is-a-gpu-buffer-and-an-l3-may-hold-state.md) settled as what crosses
the edge — and a renderer could be given all six as cheaply as three.

## Decision

**Three members and no more: `clip`, the projection to multiply a position by; `eye`, where the
camera is; and `ray`, the direction through this fragment.** `target`, `up`, `fov_y`, `near` and
`far` remain the L3's to write and no renderer can read them.

The line that decides it: **the three are a new spelling for something a renderer already had, and
the other five would be a new capability.** `clip` and `ray` are derived — the engine turns the six
numbers into `view_proj`, `basis` and `depth_range` in the one place that keeps them from drifting
— and `eye` is the one member of the six that a renderer was already being handed. Nothing about
generated WGSL changed when the notation did, and that was the notation's doing rather than luck:
the three members are the three ambients a renderer's uniform already carried, and which camera
they are is decided by the bind group the Set hands it. `karakuri-codegen` was untouched.

## Alternatives

**Expose all six, for symmetry with the type.** Rejected because it is not a spelling change: no
path carries the other five from an L3 to an L4 today, so opening them means building one, inside a
commit whose whole claim was that it changed no generated shader. That line is what kept the work
inside the checker.

## Consequences

- **The refusal is a diagnostic rather than a hole.** `view.target` is refused by name, and the
  hint lists the three members there are and says the other five are the L3's to write and
  unreadable — so a `.kir` author learns the boundary at the point of the mistake. It used to send
  them to the plan to find out why, which is worse than saying nothing.
- **This is unbuilt by decision, and that is the thing worth having written down.** A reader who
  finds five accessors missing from a six-field struct will otherwise read it as an omission and
  add them, and the cost of adding them is a data path rather than five match arms.
- What would revive it is a renderer that needs a value the derivation throws away — a shader
  wanting `fov_y` to size something in world units, say. The question then is a path from L3 to L4,
  not the spelling.

## Evidence

Commit `356fd33` (2026-08-21), which named the five and recorded them as deliberately not exposed.
The refusal and its hint are in `karakuri-ir`'s `check_camera_member`, pinned by
`a_member_a_camera_has_not_got_is_refused`.
