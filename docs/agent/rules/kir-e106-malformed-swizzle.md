---
id: KIR-E106-MALFORMED-SWIZZLE
aliases: ["E106", "KIR-E106", "kir-e106-malformed-swizzle"]
title: Malformed vector swizzle
summary: A vector swizzle expression contains invalid component characters or invalid length.
---

# Malformed vector swizzle

## Behavior

The swizzle suffix contains characters other than `xyzw` or `rgba`, or exceeds 4 components.

## Remediation

- Limit swizzle component letters strictly to coordinates (`x`, `y`, `z`, `w`) or colors (`r`, `g`, `b`, `a`).
- Do not mix coordinate and color naming in a single swizzle.
