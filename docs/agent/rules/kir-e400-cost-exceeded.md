---
id: KIR-E400-COST-EXCEEDED
aliases: ["E400", "KIR-E400", "kir-e400-cost-exceeded"]
title: Fragment cost budget exceeded
summary: Estimated static instruction cost in fragment stage exceeds safety thresholds.
---

# Fragment cost budget exceeded

## Behavior

The static instruction cost of the `fragment` stage exceeds the allocation limit (512 operations per fragment for geometry passes, 4096 operations for fullscreen passes).

## Remediation

- Reduce inner loop counts in fragment shaders.
- Simplify arithmetic expressions, raymarching step counts, or texture taps.
- Precompute complex transformations in vertex stage or pass values via parameters.
