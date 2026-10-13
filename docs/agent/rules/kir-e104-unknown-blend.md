---
id: KIR-E104-UNKNOWN-BLEND
aliases: ["E104", "KIR-E104", "kir-e104-unknown-blend"]
title: Unknown blend mode
summary: The declared blend mode in an L4 procedure is not recognized.
---

# Unknown blend mode

## Behavior

The `blend` declaration specified an unrecognized blend mode. Supported blend modes in `.kir` are strictly: `additive`, `weighted`, and `opaque`.

## Remediation

- Select a recognized blend mode: `blend additive`, `blend weighted`, or `blend opaque`.
- Consult `karakuri://doc/ir-vocabulary` for blend mode details.
