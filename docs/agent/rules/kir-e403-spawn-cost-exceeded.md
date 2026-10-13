---
id: KIR-E403-SPAWN-COST-EXCEEDED
aliases: ["E403", "KIR-E403", "kir-e403-spawn-cost-exceeded"]
title: Spawn cost budget exceeded
summary: Spawn block static instruction count exceeds budget.
---

# Spawn cost budget exceeded

## Behavior

The static instruction count for a `spawn` block exceeds the limit of 16,384 operations per spawn invocation.

## Remediation

- Simplify particle initialization logic inside the `spawn` block.
- Precompute complex initial distributions or replace multi-step loops with closed-form expressions.
