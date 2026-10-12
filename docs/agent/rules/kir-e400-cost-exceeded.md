---
id: KIR-E400-COST-EXCEEDED
aliases: ["E400", "KIR-E400", "kir-e400-cost-exceeded"]
title: Overall execution cost budget exceeded
summary: Estimated static instruction or memory cost exceeds safety thresholds.
---

# Overall execution cost budget exceeded

## Behavior

The static complexity score of the procedure exceeds the maximum instruction threshold, threatening the 60fps render budget.

## Remediation

- Reduce inner loop counts.
- Simplify arithmetic expressions and avoid redundant transcendental operations.
