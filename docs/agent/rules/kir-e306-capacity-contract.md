---
id: KIR-E306-CAPACITY-CONTRACT
aliases: ["E306", "KIR-E306", "kir-e306-capacity-contract"]
title: Invalid capacity declaration
summary: Capacity bounds or default allocation violates limits.
---

# Invalid capacity declaration

## Behavior

The capacity declaration is malformed or invalid:
- `capacity` is only permitted on and required by L1 procedures.
- Syntax is `capacity [<min>, <max>] = <default>` (no trailing semicolon).
- Bounds must satisfy `1 <= min <= default <= max`.

## Remediation

- Declare capacity in L1 as `capacity [min, max] = default` without trailing semicolon.
- Ensure `min >= 1` and `min <= default <= max`.
- Remove `capacity` declarations from non-L1 procedures.
