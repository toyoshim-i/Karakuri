---
id: KIR-E303-AMPLIFY-CONTRACT
aliases: ["E303", "KIR-E303", "kir-e303-amplify-contract"]
title: Invalid amplify factor
summary: Geometry amplification factor exceeds maximum limits or is non-constant.
---

# Invalid amplify factor

## Behavior

The `amplify` declaration violates geometry expansion contracts:
- `amplify` is only permitted on L2 procedures.
- The amplification factor must be an integer constant between 2 and 1024 inclusive (`2 <= factor <= 1024`).
- An L2 declaring `amplify` cannot also declare an input geometry slot (`uses <name> : Geometry`).

## Remediation

- Declare `amplify <N>` only in L2 procedures, with integer `2 <= N <= 1024`.
- If modifying an existing geometry stream via a slot, omit `amplify`.
