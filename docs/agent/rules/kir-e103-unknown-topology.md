---
id: KIR-E103-UNKNOWN-TOPOLOGY
aliases: ["E103", "KIR-E103", "kir-e103-unknown-topology"]
title: Unknown draw topology
summary: The declared topology in an L1 procedure is not recognized.
---

# Unknown draw topology

## Behavior

The `topology` declaration specified an unrecognized draw mode. Valid topologies are: `points`, `lines`, `triangles`, `grid`, `ribbon`, and `fullscreen`.

## Remediation

- Update the declaration to one of the valid topologies listed in `karakuri://doc/ir-vocabulary`.
- For fullscreen post-processing or raymarching, specify `topology fullscreen`.
