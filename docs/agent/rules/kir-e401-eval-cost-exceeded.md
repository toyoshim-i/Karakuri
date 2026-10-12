---
id: KIR-E401-EVAL-COST-EXCEEDED
aliases: ["E401", "KIR-E401", "kir-e401-eval-cost-exceeded"]
title: Evaluation cost limit exceeded
summary: Per-frame global evaluation instruction count exceeds budget.
---

# Evaluation cost limit exceeded

## Behavior

The per-frame evaluation pass cost exceeds the allocated budget.

## Remediation

- Move heavy computation out of the global block into the per-element or fragment stage.
- Cache repeated calculations into local variables.
