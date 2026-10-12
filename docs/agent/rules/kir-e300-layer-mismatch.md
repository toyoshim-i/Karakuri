---
id: KIR-E300-LAYER-MISMATCH
aliases: ["E300-LAYER-MISMATCH", "kir-e300-layer-mismatch"]
title: Procedure kind does not match slot target layer
summary: Procedure kind does not match the target slot layer.
---

# Procedure kind does not match slot target layer

## Behavior

An L1 procedure was written to an L4 slot layer, or an L4 procedure was written to an L1 slot layer. Layers are not interchangeable.

## Remediation

- Update the `kind` declaration inside the `.kir` file to match the target layer.
- Alternatively, specify the matching layer (`layer: "L1"` or `layer: "L4"`) in the `write_procedure` call.
