---
id: KIR-E300-LAYER-MISMATCH
aliases: ["E300-LAYER-MISMATCH", "kir-e300-layer-mismatch"]
title: Procedure kind does not match slot target layer
summary: Procedure kind does not match the target slot layer.
---

# Procedure kind does not match slot target layer

## Behavior

A procedure kind does not match the target slot where it is installed (e.g. installing an L1 generator procedure into an L4 render slot, or vice versa), or an internal stage block belongs to a different procedure kind.

## Remediation

- Update the `kind` declaration inside the `.kir` file to match the target slot kind.
- In MCP operations, install procedures into matching slots (`write_procedure` targeting the correct slot ID).
