---
id: KIR-E200-TYPE
aliases: ["E200", "KIR-E200", "kir-e200-type"]
title: Type error
summary: An expression violates typing rules.
---

# Type error

## Behavior

An operator or function was called with operands of incompatible types, or a value was assigned to a variable of a different type.

## Remediation

- Inspect the operand types reported in the diagnostic message.
- Insert explicit conversions (e.g. `float(i)` or `vec3(v.xy, 0.0)`).
