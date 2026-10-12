---
id: KIR-E305-TOPOLOGY-CONTRACT
aliases: ["E305", "KIR-E305", "kir-e305-topology-contract"]
title: Topology contract mismatch
summary: Declared topology is incompatible with stage block requirements.
---

# Topology contract mismatch

## Behavior

The procedure structure conflicts with its declared topology. For example, a `fullscreen` topology must NOT declare a `vertex` block.

## Remediation

- For `fullscreen` topology, omit the `vertex` block entirely and perform raymarching in `fragment`.
- For `lines` topology, assign both `clip` and `clip_b` in the `vertex` block.
