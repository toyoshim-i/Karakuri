---
id: KIR-E300-CONTRACT
aliases: ["E300", "KIR-E300", "kir-e300-contract"]
title: Contract violation
summary: The procedure violates interface, layout, or capability constraints.
---

# Contract violation

## Behavior

The procedure violates structural pipeline contracts, such as declaring stage blocks not permitted for its declared `kind`, invalid block combinations, or illegal stage output assignments.

## Remediation

- Review the diagnostic message for the specific contract failure.
- Consult `karakuri://doc/ir-spec` to verify valid blocks and outputs for the declared `kind`.
