---
id: KIR-E401-EVAL-COST-EXCEEDED
aliases: ["E401", "KIR-E401", "kir-e401-eval-cost-exceeded"]
title: Field evaluation cost limit exceeded
summary: Inlined field evaluations inside loops exceed budget.
---

# Field evaluation cost limit exceeded

## Behavior

Static complexity analysis determined that calling field procedures inside loops exceeds the evaluation instruction limit.

## Remediation

- Reduce loop iteration counts enclosing field evaluation calls (`field(p)`).
- Simplify the distance calculation in the referenced Field procedure.
- Hoist field evaluations outside loops where coordinates do not vary per iteration.
