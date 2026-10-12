---
id: KIR-E306-CAPACITY-CONTRACT
aliases: ["E306", "KIR-E306", "kir-e306-capacity-contract"]
title: Invalid capacity declaration
summary: Capacity bounds or default allocation violates limits.
---

# Invalid capacity declaration

## Behavior

Capacity declaration bounds are inverted (`min > max`) or default value lies outside the interval.

## Remediation

- Specify capacity as `capacity [<min>, <max>] = <default>;` where `min <= default <= max`.
- Ensure capacity does not exceed the hardware particle ceiling (1,048,576).
