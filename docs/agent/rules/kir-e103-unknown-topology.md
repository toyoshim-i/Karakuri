---
id: KIR-E103-UNKNOWN-TOPOLOGY
aliases: ["E103", "KIR-E103", "kir-e103-unknown-topology"]
title: Unknown draw topology
summary: The declared topology in an L1 procedure is not recognized.
---

# Unknown draw topology

## Behavior

The `topology` declaration in an L1 header specified an unrecognized draw mode. Topologies declared in L1 headers must be one of: `points`, `lines`, `triangles`, `grid`, or `ribbon`. (Note: `fullscreen` is an L4 rendering configuration without an L1 topology, not an L1 source topology).

## Remediation

- Update the declaration to one of the valid topologies: `topology points`, `topology lines`, `topology triangles`, `topology grid`, or `topology ribbon`.
- Topologies sharing connectivity across elements (`triangles`, `grid`, `ribbon`) require static sources (no `spawn` block or `kill()` calls).
