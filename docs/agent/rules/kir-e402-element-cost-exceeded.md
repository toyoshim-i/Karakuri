---
id: KIR-E402-ELEMENT-COST-EXCEEDED
aliases: ["E402", "KIR-E402", "kir-e402-element-cost-exceeded"]
title: Per-element cost limit exceeded
summary: Vertex or element processing instruction count exceeds budget.
---

# Per-element cost limit exceeded

## Behavior

The element update kernel contains too many instructions per particle/element.

## Remediation

- Simplify noise calculations (e.g. fewer octaves in fbm).
- Replace expensive trigonometric calls with approximations or direct coordinates.
