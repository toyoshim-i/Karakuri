---
id: KIR-E307-SEED-CONTRACT
aliases: ["E307", "KIR-E307", "kir-e307-seed-contract"]
title: Seed contract error
summary: Ambient seed access is not permitted in this procedure kind or stage.
---

# Seed contract error

## Behavior

The `seed` ambient signal was accessed in a procedure kind or stage where element identities do not exist. `seed` is restricted to element-bearing procedures (L1, L2, L3, or L4 with an element source) and is forbidden in `Field` procedures and fullscreen L4/L5 procedures.

## Remediation

- Use `seed` only within element-bearing procedures.
- In Field or fullscreen L4/L5 procedures, derive pseudo-randomness from spatial coordinates (`point`, `point_coord`, `clip`) or time (`t`).
