---
id: KIR-E304-BLEND-CONTRACT
aliases: ["E304", "KIR-E304", "kir-e304-blend-contract"]
title: Blend contract violation
summary: Blend configuration is missing, declared on an incompatible kind, or specifies an invalid mode.
---

# Blend contract violation

## Behavior

A procedure's blend mode declaration violates stage contracts:
- `blend` is required on L4 procedures.
- `blend` is forbidden on procedures other than L4 (such as L1, L2, L3, Field, L5).
- Supported blend modes are strictly `additive`, `weighted`, and `opaque`.

## Remediation

- For L4 procedures, declare `blend additive`, `blend weighted`, or `blend opaque`.
- Remove `blend` declarations from non-L4 procedures.
