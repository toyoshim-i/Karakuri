---
id: KIR-E202-INVALID-SWIZZLE
aliases: ["E202", "KIR-E202", "kir-e202-invalid-swizzle"]
title: Invalid swizzle on type
summary: Swizzling was attempted on a non-vector type or out-of-bounds component.
---

# Invalid swizzle on type

## Behavior

Swizzle access was attempted on a non-vector type (such as scalar `float` or `int`), or accessed components beyond the vector's dimensionality (e.g. `.z` on a `vec2`, or `.w` on a `vec3`).

## Remediation

- Verify that the target expression is a vector (`vec2`, `vec3`, or `vec4`).
- Ensure swizzle components do not exceed vector dimensions (`.xy` for `vec2`, `.xyz` for `vec3`, `.xyzw` for `vec4`).
