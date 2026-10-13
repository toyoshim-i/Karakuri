---
id: KIR-E102-UNKNOWN-KIND
aliases: ["E102", "KIR-E102", "kir-e102-unknown-kind"]
title: Unknown procedure kind
summary: Declared procedure kind is not recognized by the compiler.
---

# Unknown procedure kind

## Behavior

The procedure header specified an invalid `kind` identifier. Valid procedure kinds are:
- `L1` (source generator / particle simulation)
- `L2` (geometry deformer / modifier)
- `L3` (geometry processor / camera coordinator)
- `Field` (signed distance / scalar field function)
- `L4` (visual renderer / rasterizer)
- `L5` (post-processing effect / screen pass)

## Remediation

- Change the header declaration to one of the supported kinds (`kind L1`, `kind L2`, `kind L3`, `kind Field`, `kind L4`, or `kind L5`).
- Ensure the procedure structure and stage blocks conform to the declared kind.
