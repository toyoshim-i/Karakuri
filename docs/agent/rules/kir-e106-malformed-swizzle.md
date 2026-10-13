---
id: KIR-E106-MALFORMED-SWIZZLE
aliases: ["E106", "KIR-E106", "kir-e106-malformed-swizzle"]
title: Malformed vector swizzle
summary: A vector swizzle expression contains invalid component characters or invalid length.
---

# Malformed vector swizzle

## Behavior

The swizzle suffix contains characters other than `x`, `y`, `z`, `w`, or specifies more than 4 components. (In `.kir`, swizzles strictly use coordinate components `x`, `y`, `z`, `w`; color letters `r`, `g`, `b`, `a` are not supported).

## Remediation

- Limit swizzle component letters strictly to coordinate axes: `x`, `y`, `z`, `w`.
- Ensure the swizzle length is between 1 and 4 components (e.g. `.x`, `.xy`, `.xyz`, `.xyzw`).
