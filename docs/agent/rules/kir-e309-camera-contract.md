---
id: KIR-E309-CAMERA-CONTRACT
aliases: ["E309", "KIR-E309", "kir-e309-camera-contract"]
title: Camera contract error
summary: Camera slot binding or camera ambient access is malformed.
---

# Camera contract error

## Behavior

Camera declarations or ambient accesses violate pipeline contracts:
- Declaring `uses <name> : Camera` is only permitted in L4 procedures (at most one camera slot).
- L3 procedures require a `camera` block.
- Camera ambients (`camera`, `eye`, `ray`) require appropriate stage blocks (`camera` in vertex; `eye` and `ray` in fullscreen fragment).

## Remediation

- For 3D camera projection in L4, declare at most one camera slot: `uses cam : Camera`.
- In L3 procedures, include a `camera { ... }` block.
- Read camera ambients only within the stages where they are defined.
