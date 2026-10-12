---
id: KIR-E102-UNKNOWN-KIND
aliases: ["E102", "KIR-E102", "kir-e102-unknown-kind"]
title: Unknown procedure kind
summary: Declared procedure kind is not recognized by the compiler.
---

# Unknown procedure kind

## Behavior

The procedure header specified an invalid `kind` identifier. In `.kir` v0.2, valid procedure kinds are `L1` (particle/geometry generator) and `L4` (visual renderer).

## Remediation

- Change the header declaration to `kind L1` or `kind L4`.
- Ensure the procedure structure conforms to the declared kind.
