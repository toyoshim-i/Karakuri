---
id: KIR-E203-TYPE-MISMATCH
aliases: ["E203", "KIR-E203", "kir-e203-type-mismatch"]
title: Type mismatch
summary: Operands or assignment targets have incompatible types.
---

# Type mismatch

## Behavior

A binary operation or assignment attempted to combine different numeric types (e.g. adding `int` and `float`) without explicit casting.

## Remediation

- Explicitly convert integers to floats using `float(...)` or floats to integers using `int(...)`.
- Construct matching vector sizes using `vec2(...)`, `vec3(...)`, or `vec4(...)` constructors.
