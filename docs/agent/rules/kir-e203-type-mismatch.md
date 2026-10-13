---
id: KIR-E203-TYPE-MISMATCH
aliases: ["E203", "KIR-E203", "kir-e203-type-mismatch"]
title: Type mismatch
summary: Operands or assignment targets have incompatible types.
---

# Type mismatch

## Behavior

A binary operation or assignment attempted to combine different numeric types (e.g. adding `int` and `float`) without explicit conversion, or assigned a value to a variable or stage output of a differing type.

## Remediation

- Explicitly convert numeric types using `float(...)` or `int(...)`.
- Construct matching vector types using `vec2(...)`, `vec3(...)`, or `vec4(...)`.
- Ensure output assignments match required stage output types (e.g. `color = vec4(...)`).
