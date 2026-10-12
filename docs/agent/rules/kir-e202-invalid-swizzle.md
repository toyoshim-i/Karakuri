---
id: KIR-E202-INVALID-SWIZZLE
aliases: ["E202", "KIR-E202", "kir-e202-invalid-swizzle"]
title: Invalid swizzle on type
summary: Swizzling was attempted on a non-vector type or out-of-bounds component.
---

# Invalid swizzle on type

## Behavior

Swizzle access was attempted on a scalar float or integer, or accessed components beyond the vector's dimension (e.g. `.z` on a `vec2`).

## Remediation

- Verify that the target expression is a vector.
- Ensure swizzle components do not exceed the vector dimensionality (`.xy` for `vec2`, `.xyz` for `vec3`).
