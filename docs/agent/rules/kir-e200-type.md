---
id: KIR-E200-TYPE
aliases: ["E200", "KIR-E200", "kir-e200-type"]
title: Type error
summary: An expression violates typing rules.
---

# Type error

## Behavior

An operator or function was called with operands of incompatible types, or a value was used where a different type or shape was required.

## Remediation

- Inspect operand types reported in the diagnostic message.
- Use explicit type constructors (`float(...)`, `int(...)`, `vec2(...)`, `vec3(...)`, `vec4(...)`, `mat4(...)`).
- Consult `karakuri://doc/ir-vocabulary` to verify function argument types.
