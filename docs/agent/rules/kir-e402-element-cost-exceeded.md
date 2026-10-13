---
id: KIR-E402-ELEMENT-COST-EXCEEDED
aliases: ["E402", "KIR-E402", "kir-e402-element-cost-exceeded"]
title: Per-element cost limit exceeded
summary: Vertex or element processing instruction count exceeds budget.
---

# Per-element cost limit exceeded

## Behavior

The static instruction count for an `element`, `deform`, or `vertex` block exceeds the limit of 4096 operations per element.

## Remediation

- Simplify arithmetic and transcendental functions (e.g. fewer noise octaves or trigonometric calls).
- Reduce loop iteration counts in element and vertex stages.
- Split heavy calculations across multiple pipeline stages (e.g. generator followed by modifier).
