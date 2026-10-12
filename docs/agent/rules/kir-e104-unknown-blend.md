---
id: KIR-E104-UNKNOWN-BLEND
aliases: ["E104", "KIR-E104", "kir-e104-unknown-blend"]
title: Unknown blend mode
summary: The declared blend mode in an L4 procedure is not recognized.
---

# Unknown blend mode

## Behavior

The `blend` declaration specified an unrecognized blend mode. Supported blend modes are: `additive`, `weighted`, `alpha`, `multiply`, and `screen`.

## Remediation

- Select a recognized blend mode from `karakuri://doc/ir-vocabulary`.
- For additive particle accumulation, declare `blend additive`.
