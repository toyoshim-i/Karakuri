---
id: KIR-E403-SPAWN-COST-EXCEEDED
aliases: ["E403", "KIR-E403", "kir-e403-spawn-cost-exceeded"]
title: Spawn rate budget exceeded
summary: Element generation rate exceeds memory allocation bandwidth.
---

# Spawn rate budget exceeded

## Behavior

The procedure attempts to instantiate more elements per frame than available buffer allocation allows.

## Remediation

- Reduce particle emission rates.
- Lower the maximum capacity declared in the `capacity` block.
