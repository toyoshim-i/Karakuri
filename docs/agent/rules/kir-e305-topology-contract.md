---
id: KIR-E305-TOPOLOGY-CONTRACT
aliases: ["E305", "KIR-E305", "kir-e305-topology-contract"]
title: Topology contract mismatch
summary: Declared topology is incompatible with stage block requirements.
---

# Topology contract mismatch

## Behavior

The procedure structure conflicts with its declared topology:
- L1 procedures require a `topology` declaration (`points`, `lines`, `triangles`, `grid`, `ribbon`).
- Connected topologies (`triangles`, `grid`, `ribbon`) require static sources (they cannot contain `spawn` blocks or `kill()` calls).
- Fullscreen L4 procedures (no `vertex` block) cannot consume element attributes.
- In L4 with `lines` topology, the `vertex` block must assign both `clip` and `clip_b`.

## Remediation

- For L1, specify one of `points`, `lines`, `triangles`, `grid`, or `ribbon`.
- For connected topologies (`triangles`, `grid`, `ribbon`), use static element generation without dynamic spawn.
- For fullscreen L4 procedures, omit `vertex` and `consumes`.
